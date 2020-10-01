pub(crate) mod optimizer;
use crate::{
    lazy::{prelude::*, utils},
    prelude::*,
};
use arrow::datatypes::DataType;
use fnv::FnvHashSet;
use std::cell::RefCell;
use std::{fmt, rc::Rc};

#[derive(Clone, Debug)]
pub enum ScalarValue {
    Null,
    /// A binary true or false.
    Boolean(bool),
    /// A UTF8 encoded string type.
    Utf8(String),
    /// An unsigned 8-bit integer number.
    UInt8(u8),
    /// An unsigned 16-bit integer number.
    UInt16(u16),
    /// An unsigned 32-bit integer number.
    UInt32(u32),
    /// An unsigned 64-bit integer number.
    UInt64(u64),
    /// An 8-bit integer number.
    Int8(i8),
    /// A 16-bit integer number.
    Int16(i16),
    /// A 32-bit integer number.
    Int32(i32),
    /// A 64-bit integer number.
    Int64(i64),
    /// A 32-bit floating point number.
    Float32(f32),
    /// A 64-bit floating point number.
    Float64(f64),
}

impl ScalarValue {
    /// Getter for the `DataType` of the value
    pub fn get_datatype(&self) -> DataType {
        match *self {
            ScalarValue::Boolean(_) => DataType::Boolean,
            ScalarValue::UInt8(_) => DataType::UInt8,
            ScalarValue::UInt16(_) => DataType::UInt16,
            ScalarValue::UInt32(_) => DataType::UInt32,
            ScalarValue::UInt64(_) => DataType::UInt64,
            ScalarValue::Int8(_) => DataType::Int8,
            ScalarValue::Int16(_) => DataType::Int16,
            ScalarValue::Int32(_) => DataType::Int32,
            ScalarValue::Int64(_) => DataType::Int64,
            ScalarValue::Float32(_) => DataType::Float32,
            ScalarValue::Float64(_) => DataType::Float64,
            ScalarValue::Utf8(_) => DataType::Utf8,
            _ => panic!("Cannot treat {:?} as scalar value", self),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Operator {
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    Plus,
    Minus,
    Multiply,
    Divide,
    Modulus,
    And,
    Or,
    Not,
    Like,
    NotLike,
}

// https://stackoverflow.com/questions/1031076/what-are-projection-and-selection
#[derive(Clone)]
pub enum LogicalPlan {
    // filter on a boolean mask
    Selection {
        input: Box<LogicalPlan>,
        predicate: Expr,
    },
    CsvScan {
        path: String,
        schema: Schema,
        has_header: bool,
        delimiter: Option<u8>,
    },
    DataFrameScan {
        df: Rc<RefCell<DataFrame>>,
        schema: Schema,
    },
    // vertical selection
    Projection {
        expr: Vec<Expr>,
        input: Box<LogicalPlan>,
        schema: Schema,
    },
    Sort {
        input: Box<LogicalPlan>,
        column: String,
        reverse: bool,
    },
    Aggregate {
        input: Box<LogicalPlan>,
        keys: Rc<Vec<String>>,
        aggs: Vec<Expr>,
        schema: Schema,
    },
    Join {
        input_left: Box<LogicalPlan>,
        input_right: Box<LogicalPlan>,
        schema: Schema,
        how: JoinType,
        left_on: Rc<String>,
        right_on: Rc<String>,
    },
}

impl fmt::Debug for LogicalPlan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use LogicalPlan::*;
        match self {
            Selection { predicate, input } => write!(f, "Filter\n\t{:?} {:?}", predicate, input),
            CsvScan { path, .. } => write!(f, "CSVScan {}", path),
            DataFrameScan { schema, .. } => write!(
                f,
                "TABLE: {:?}",
                schema
                    .fields()
                    .iter()
                    .map(|f| f.name())
                    .take(4)
                    .collect::<Vec<_>>()
            ),
            Projection { expr, input, .. } => write!(f, "SELECT {:?} \nFROM\n{:?}", expr, input),
            Sort { input, column, .. } => write!(f, "Sort\n\t{:?}\n{:?}", column, input),
            Aggregate { keys, aggs, .. } => write!(f, "Aggregate\n\t{:?} BY {:?}", aggs, keys),
            Join {
                input_left,
                input_right,
                left_on,
                right_on,
                ..
            } => write!(
                f,
                "JOIN ({:?}) WITH ({:?}) ON (left: {} right: {})",
                input_left, input_right, left_on, right_on
            ),
        }
    }
}

pub struct LogicalPlanBuilder(LogicalPlan);

impl LogicalPlan {
    pub(crate) fn schema(&self) -> &Schema {
        use LogicalPlan::*;
        match self {
            DataFrameScan { schema, .. } => schema,
            Selection { input, .. } => input.schema(),
            CsvScan { schema, .. } => schema,
            Projection { schema, .. } => schema,
            Sort { input, .. } => input.schema(),
            Aggregate { schema, .. } => schema,
            Join { schema, .. } => schema,
        }
    }
    pub fn describe(&self) -> String {
        format!("{:#?}", self)
    }
}

impl From<LogicalPlan> for LogicalPlanBuilder {
    fn from(lp: LogicalPlan) -> Self {
        LogicalPlanBuilder(lp)
    }
}

impl LogicalPlanBuilder {
    pub fn scan_csv() -> Self {
        todo!()
    }

    pub fn project(self, expr: Vec<Expr>) -> Self {
        let schema = utils::expressions_to_schema(&expr, self.0.schema());
        LogicalPlan::Projection {
            expr,
            input: Box::new(self.0),
            schema,
        }
        .into()
    }

    /// Apply a filter
    pub fn filter(self, predicate: Expr) -> Self {
        LogicalPlan::Selection {
            predicate,
            input: Box::new(self.0),
        }
        .into()
    }

    pub fn groupby(self, keys: Rc<Vec<String>>, aggs: Vec<Expr>) -> Self {
        let current_schema = self.0.schema();

        let fields = keys
            .iter()
            .map(|name| current_schema.field_with_name(name).unwrap().clone())
            .collect::<Vec<_>>();

        let schema1 = Schema::new(fields);

        let schema2 = utils::expressions_to_schema(&aggs, self.0.schema());
        let schema = Schema::try_merge(&[schema1, schema2]).unwrap();

        LogicalPlan::Aggregate {
            input: Box::new(self.0),
            keys,
            aggs,
            schema,
        }
        .into()
    }

    pub fn build(self) -> LogicalPlan {
        self.0
    }

    pub fn from_existing_df(df: DataFrame) -> Self {
        let schema = df.schema();
        LogicalPlan::DataFrameScan {
            df: Rc::new(RefCell::new(df)),
            schema,
        }
        .into()
    }

    pub fn sort(self, by_column: String, reverse: bool) -> Self {
        LogicalPlan::Sort {
            input: Box::new(self.0),
            column: by_column,
            reverse,
        }
        .into()
    }

    pub fn join(
        self,
        other: LogicalPlan,
        how: JoinType,
        left_on: Rc<String>,
        right_on: Rc<String>,
    ) -> Self {
        let schema_left = self.0.schema();
        let schema_right = other.schema();

        let mut set = FnvHashSet::default();

        for f in schema_left.fields() {
            set.insert(f.clone());
        }

        for f in schema_right.fields() {
            if set.contains(f) {
                let field = Field::new(
                    &format!("{}_right", f.name()),
                    f.data_type().clone(),
                    f.is_nullable(),
                );
                set.insert(field);
            } else {
                set.insert(f.clone());
            }
        }
        let schema = Schema::new(set.into_iter().collect());

        LogicalPlan::Join {
            input_left: Box::new(self.0),
            input_right: Box::new(other),
            how,
            schema,
            left_on,
            right_on,
        }
        .into()
    }
}

#[derive(Clone, Debug)]
pub enum JoinType {
    Left,
    Inner,
    Outer,
}

#[cfg(test)]
mod test {
    use crate::lazy::prelude::*;
    use crate::lazy::tests::get_df;
    use crate::prelude::*;

    fn compare_plans(lf: &LazyFrame) {
        println!("LOGICAL PLAN\n\n{}\n", lf.describe_plan());
        println!(
            "OPTIMIZED LOGICAL PLAN\n\n{}\n",
            lf.describe_optimized_plan()
        );
    }

    #[test]
    fn test_lazy_logical_plan_schema() {
        let df = get_df();
        let lp = df
            .clone()
            .lazy()
            .select(&[col("variety").alias("foo")])
            .logical_plan;

        println!("{:#?}", lp.schema().fields());
        assert!(lp.schema().field_with_name("foo").is_ok());

        let lp = df
            .lazy()
            .groupby("variety")
            .agg(vec![col("sepal.width").agg_min()])
            .logical_plan;
        println!("{:#?}", lp.schema().fields());
        assert!(lp.schema().field_with_name("sepal.width_min").is_ok());
    }

    #[test]
    fn test_lazy_logical_plan_join() {
        let left = df!("days" => &[0, 1, 2, 3, 4],
        "temp" => [22.1, 19.9, 7., 2., 3.]
        )
        .unwrap();

        let right = df!(
        "days" => &[1, 2],
        "rain" => &[0.1, 0.2]
        )
        .unwrap();

        let lf = left
            .lazy()
            .left_join(right.lazy(), "days", "days")
            .select(&[col("temp")]);

        compare_plans(&lf);

        let df = lf.collect().unwrap();
        println!("{:?}", df);

        assert!(false)
    }
}