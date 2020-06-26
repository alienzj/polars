//! # Polars DataFrames in Rust
//!
//! # WIP
//!
//! ## Read csv
//!
//! ```
//! use polars::prelude::*;
//! use std::fs::File;
//!
//! fn example() -> Result<DataFrame> {
//!     let file = File::open("iris.csv").expect("could not open file");
//!
//!     CsvReader::new(file)
//!             .infer_schema(None)
//!             .has_header(true)
//!             .finish()
//! }
//! ```
//!
//! ## Join
//!
//! ```
//! use polars::prelude::*;
//!
//! // Create first df.
//! let s0 = Series::init("days", [0, 1, 2, 3, 4].as_ref());
//! let s1 = Series::init("temp", [22.1, 19.9, 7., 2., 3.].as_ref());
//! let temp = DataFrame::new_from_columns(vec![s0, s1]).unwrap();
//!
//! // Create second df.
//! let s0 = Series::init("days", [1, 2].as_ref());
//! let s1 = Series::init("rain", [0.1, 0.2].as_ref());
//! let rain = DataFrame::new_from_columns(vec![s0, s1]).unwrap();
//!
//! // Left join on days column.
//! let joined = temp.left_join(&rain, "days", "days");
//! println!("{}", joined.unwrap())
//! ```
//!
//! ## GroupBy
//!
//! ```
//! use polars::prelude::*;
//! fn groupby_sum(df: &DataFrame) -> Result<DataFrame> {
//!     df.groupby("column_name")?
//!     .select("agg_column_name")
//!     .sum()
//! }
//! ```
//!
//! ## Arithmetic
//! ```
//! use polars::prelude::*;
//! let s: Series = [1, 2, 3].iter().collect();
//! let s_squared = &s * &s;
//! ```
//!
//! ## Rust iterators
//!
//! ```
//! use polars::prelude::*;
//!
//! let s: Series = [1, 2, 3].iter().collect();
//! let s_squared: Series = s.i32()
//!      .expect("datatype mismatch")
//!      .into_iter()
//!      .map(|optional_v| {
//!          match optional_v {
//!              Some(v) => Some(v * v),
//!              None => None, // null value
//!          }
//!  }).collect();
//! ```
//!
//! ## Comparison
//!
//! ```
//! use polars::prelude::*;
//! use itertools::Itertools;
//! let s = Series::init("dollars", [1, 2, 3].as_ref());
//! let mask = s.eq(1).expect("could not compare types");
//! let valid = [true, false, false].iter();
//! assert!(mask
//!     .into_iter()
//!     .map(|opt_bool| opt_bool.unwrap()) // option, because series can be null
//!     .zip(valid)
//!     .all(|(a, b)| a == *b))
//! ```
//!
//! Read more in the [DataFrame](frame/struct.DataFrame.html) and [Series](/series/series/index.html)
//! modules.
#![allow(dead_code)]
#![feature(iterator_fold_self)]
pub mod error;
pub mod series {
    pub(crate) mod aggregate;
    pub(crate) mod arithmetic;
    pub mod chunked_array;
    mod comparison;
    pub(crate) mod iterator;
    pub mod series;
}
pub mod datatypes;
mod fmt;
pub mod frame;
pub mod prelude;
pub mod testing;