test_coverage:
# --target-dir prevents it from clearing the shared target dir
	cargo tarpaulin \
		--workspace \
		--out Lcov \
		--frozen \
		--locked \
		--target-dir=target_tarpaulin
