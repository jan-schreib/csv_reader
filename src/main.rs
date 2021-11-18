use std::error::Error;
use std::{env, process};
use transaction::Transaction;

mod client;
mod transaction;

// Read and parse the csv file. Returns an error if the file can not be read or if there is a parsing error.
// Reads the whole file bevor returning transactions.
fn read_input_file(path: &str) -> Result<Vec<Transaction>, Box<dyn Error>> {
    let mut txs = Vec::new();

    // trim all whitespace and allow flexible fields in records to be able to parse
    // all possible transaction types.
    let mut rdr = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .flexible(true)
        .from_path(path)?;
    for result in rdr.deserialize() {
        let record: Transaction = result?;
        txs.push(record);
    }
    Ok(txs)
}

// The application read, parse, process the input file that is given as command line argument
// and write the result to stdout.
// On command line errors the program will exit with exitcode 1.
//
// Command line example:
// ./csv_read <input.csv>
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        println!("Usage: ./csvread input.csv");
        process::exit(1);
    }

    let filename = &args[1];
    let txs = match read_input_file(filename) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1);
        }
    };

    let clients = Transaction::handle_transactions(txs);

    let mut wtr = csv::WriterBuilder::new().from_writer(std::io::stdout());

    for c in clients {
        wtr.serialize(c)?;
    }

    wtr.flush()?;

    Ok(())
}
