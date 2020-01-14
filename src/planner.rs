// Types of locks required by a statement for a table
enum Lock {
	Read,
	Write,

	// Separate to distinguish a statement needing a plain insert/update/delete
	// on a table and a statement performing aggregation on a table and then
	// deleting based on that. The former can be run more concurrently with some
	// in some cases.
	Both,
}
