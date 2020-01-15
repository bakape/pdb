use serde::{Deserialize, Serialize};

// Select statement builder
#[derive(Serialize, Deserialize, Clone)]
pub struct SelectBuilder {
	// Source table or statement
	// TODO: enum
	table: String,

	// Columns to return from table.
	// Empty vector means all available columns should be returned.
	columns: Vec<Column>,
}

#[derive(Serialize, Deserialize, Clone)]
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
        as_item! {
			// Supported value types usable in the query builder

			#[derive(Serialize, Deserialize, Clone)]
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
				Vec_u8(Vec<u8>),
			}
		}

		$(
			impl From<$variant> for Value {
				fn from(v: $variant) -> Self {
					Self::$variant(v)
				}
			}
		)+
    };
}

macro_rules! as_item {
	($i:item) => {
			$i
	};
}

build_value_enum! {
	i8 i16 i32 i64 i128 isize
	u8 u16 u32 u64 u128
	f32 f64
	char
	bool
	String

	// TODO: More types (see Postgres for inspiration)
}

impl From<&str> for Value {
	fn from(v: &str) -> Self {
		Self::Column(v.into())
	}
}

impl From<SelectBuilder> for Value {
	fn from(v: SelectBuilder) -> Self {
		Self::Expression(v)
	}
}

impl From<Vec<u8>> for Value {
	fn from(v: Vec<u8>) -> Self {
		Self::Vec_u8(v)
	}
}

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
	fn filter_construction() -> Result<(), String> {
		Filter::new("user", Comparator::Eq, 20_u64);
		Filter::new("user", Comparator::Eq, select("id", &["article"])?);
		Ok(())
	}
}
