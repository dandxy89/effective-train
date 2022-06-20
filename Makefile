.PHONY: clean run test

clean:
	rm -rf accounts.csv
	rm -rf transaction_processor.log

test:
	cargo test

run: clean
	cargo run -- resources/demo.csv > accounts.csv
