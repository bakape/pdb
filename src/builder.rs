use chrono::{Date, DateTime, Duration};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::net::IpAddr;
use uuid::Uuid;

// Select statement builder
#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct SelectBuilder {
	// Source table or statement
	// TODO: enum
	table: String,

	// Columns to return from table.
	// Empty vector means all available columns should be returned.
	columns: Vec<Column>,
}

#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq)]
struct Column {
	name: String,
	alias: Option<String>,
}

// Build new select statement from a table, selecting the passed columns.
// Empty column list denotes all available columns should be returned.
// Each column may include a space-separated alias. Ex.: "user u"
pub fn select(
	table: impl Into<String>,
	columns: impl IntoIterator<Item = impl AsRef<str>>,
) -> Result<SelectBuilder, String> {
	Ok(SelectBuilder {
		table: table.into(),
		columns: {
			let it = columns.into_iter();
			let mut res = Vec::with_capacity(match it.size_hint().1 {
				Some(s) => s,
				None => 0,
			});
			for c in it {
				let mut s = c.as_ref().split_ascii_whitespace();
				res.push(Column {
					name: match s.next() {
						Some(s) => s.into(),
						None => return Err("empty column name".into()),
					},
					alias: s.next().map(|x| x.into()),
				})
			}
			res
		},
	})
}

macro_rules! build_value_enum {
    ($($variant:tt)+) => {
		// Supported value types usable in the query builder
		#[derive(Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq)]
		#[allow(non_camel_case_types)]
		pub enum Value {
			// Lack of value
			Null,

			// Value inside the current row's column by name or alias
			Column(String),

			// Result of SQL expression. Note that for Comparators other
			// than In, only the first result of the expression will be
			// used, if any.
			Expression(SelectBuilder),

			$(
				// Constant value
				$variant($variant),
			)+

			// Constant value
			f32(F32),

			// Constant value
			f64(F64),

			// Constant value
			UUID([u8; 16]),

			// Constant vector of constant values
			Vec(Vec<Value>),

			// Constant BTreeSet of constant values
			Set(BTreeSet<Value>),

			// Constant BTreeMap of constant values
			Map(BTreeMap<Value, Value>),
		}

		$( impl_value_conversion! {$variant} )+
    };
}

impl Default for Value {
	fn default() -> Self {
		Self::Null
	}
}

macro_rules! impl_value_conversion {
	($type:ty, $variant:ident) => {
		impl_value_conversion! {
			$type,
			|v: $type| Self::$variant(v.into())
		}
	};
	($type:ty, $convert:expr) => {
		impl From<$type> for Value {
			fn from(v: $type) -> Self {
				$convert(v)
			}
		}
	};
	($type:tt) => {
		impl_value_conversion! {$type, $type}
	};
}

build_value_enum! {
	bool

	i8 i16 i32 i64 i128 isize
	u8 u16 u32 u64 u128

	char
	String

	IpAddr

	// Duration
	// Date
	// DateTime
}

macro_rules! impl_wrapper_tuple {
	($tuple:tt, $inner:tt) => {
		define_tuple! {$tuple, $inner}

		impl From<$inner> for $tuple {
			fn from(v: $inner) -> Self {
				Self(v)
			}
		}

		impl Into<$inner> for $tuple {
			fn into(self) -> $inner {
				self.0
			}
		}

		impl std::cmp::PartialEq for $tuple {
			fn eq(&self, other: &$tuple) -> bool {
				self.0.to_le_bytes() == other.0.to_le_bytes()
			}
		}

		impl std::cmp::Eq for $tuple {}

		impl std::cmp::PartialOrd for $tuple {
			fn partial_cmp(
				&self,
				other: &$tuple,
			) -> Option<std::cmp::Ordering> {
				self.0.to_le_bytes().partial_cmp(&other.0.to_le_bytes())
			}
		}

		impl std::cmp::Ord for $tuple {
			fn cmp(&self, other: &$tuple) -> std::cmp::Ordering {
				self.0.to_le_bytes().cmp(&other.0.to_le_bytes())
			}
		}
	};
}

macro_rules! define_tuple {
	($ident:ident, $inner:tt) => {
		// Wrapper for hashing using as a hash collection key
		#[derive(Serialize, Deserialize, Clone)]
		pub struct $ident($inner);
	};
}

impl_wrapper_tuple! {F32, f32}
impl_wrapper_tuple! {F64, f64}

impl_value_conversion! {&str, Column}
impl_value_conversion! {SelectBuilder, Expression}

impl From<Uuid> for Value {
	fn from(v: Uuid) -> Self {
		Self::UUID(*v.as_bytes())
	}
}

macro_rules! impl_value_from_linear {
	($container:ident, $variant:ident) => {
		impl<T> From<$container<T>> for Value
		where
			T: Into<Value>,
		{
			fn from(v: $container<T>) -> Self {
				Value::$variant(v.into_iter().map(|v| v.into()).collect())
			}
		}
	};
}

impl_value_from_linear! {Vec, Vec}
impl_value_from_linear! {BTreeSet, Set}
impl_value_from_linear! {HashSet, Set}

macro_rules! impl_value_from_map {
	($container:ident, $variant:ident) => {
		impl<K, V> From<$container<K, V>> for Value
		where
			K: Into<Value> + Eq + PartialEq,
			V: Into<Value>,
		{
			fn from(v: $container<K, V>) -> Self {
				Value::Map(
					v.into_iter().map(|(k, v)| (k.into(), v.into())).collect(),
				)
			}
		}
	};
}

impl_value_from_map! {BTreeMap, Map}
impl_value_from_map! {HashMap, Map}

#[derive(Serialize, Deserialize, Clone)]
pub enum Comparator {
	// Equal
	Eq,

	// Greater than
	Gt,

	// Greater than or equal
	Gte,

	// Less than
	Lt,

	// Less than or equal
	Lte,

	// In the subset on the right hand side
	In,
}

#[derive(Serialize, Deserialize, Clone)]
enum FilterInner {
	Simple {
		// Left hand side value
		lhs: Value,

		// Comparator for the two values
		comp: Comparator,

		// Right hand side value
		rhs: Value,
	},
	Combined {
		// and/or combination mode
		and: bool,

		// Left hand side value
		lhs: Box<Filter>,

		// Filter to combine with
		rhs: Box<Filter>,
	},
}

// Can be modified and combined using ! (not), + (and), | (or) operators
#[derive(Serialize, Deserialize, Clone)]
pub struct Filter {
	// SQL `not` equivalent
	inverted: bool,

	inner: FilterInner,
}

impl Filter {
	pub fn new(
		lhs: impl Into<Value>,
		comp: Comparator,
		rhs: impl Into<Value>,
	) -> Self {
		Self {
			inverted: false,
			inner: FilterInner::Simple {
				lhs: lhs.into(),
				comp: comp,
				rhs: rhs.into(),
			},
		}
	}

	fn combine(self, rhs: Self, and: bool) -> Self {
		Self {
			inverted: false,
			inner: FilterInner::Combined {
				lhs: self.into(),
				and: and,
				rhs: rhs.into(),
			},
		}
	}
}

impl std::ops::Add for Filter {
	type Output = Self;

	fn add(self, rhs: Self) -> Self {
		self.combine(rhs, true)
	}
}

impl std::ops::BitOr for Filter {
	type Output = Self;

	fn bitor(self, rhs: Self) -> Self {
		self.combine(rhs, false)
	}
}

impl std::ops::Not for Filter {
	type Output = Self;

	fn not(mut self) -> Self {
		self.inverted = !self.inverted;
		self
	}
}

impl SelectBuilder {
	// Apply filter to current row set
	fn filter(self, f: Filter) -> SelectBuilder {
		todo!()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	// Simply assert this compiles
	#[test]
	#[allow(unused)]
	fn filter_construction() -> Result<(), String> {
		!(Filter::new("user", Comparator::Eq, 20_u64)
			+ Filter::new("user", Comparator::Eq, select("id", &["article"])?))
			| Filter::new("bucket", Comparator::Eq, vec![1_i32]);
		Ok(())
	}
}
