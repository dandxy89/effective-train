# effective-train

## Usage

    cargo build
    cargo run -- resources/tx-demo.csv > accounts.csv

Running the above command will write from stdout into a file and write logs to a file called `transaction_processor.log`.

## Testing

    cargo test

## Possible Improvements

- Documentation - adding thorough documentation before peer-review
- Requirements - gain a deeper understanding to allow the code be refactored to support a Database / Data Store, integration with a HttpServer and init with a CI pipeline
- Load testing - As part of the pipeline I'd expect to verify the performance of the application
- Test case - Could be consolidated / refactored to be less verbose
