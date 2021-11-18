use serde::{Deserialize, Serialize, Serializer};
use std::error::Error;
use std::{env, process};

#[derive(Deserialize, Debug, Clone)]
struct Transaction {
    #[serde(rename(deserialize = "type"))]
    tx_type: String,
    #[serde(rename(deserialize = "client"))]
    client_id: u16,
    #[serde(rename(deserialize = "tx"))]
    tx_id: u32,
    amount: Option<f64>,
    #[serde(skip_deserializing)]
    disputed: bool,
}

impl Transaction {
    fn amount(&self) -> f64 {
        self.amount.unwrap_or(0.0)
    }
}

#[derive(Serialize, Debug)]
struct Client {
    #[serde(rename(serialize = "client"))]
    client_id: u16,
    #[serde(rename(serialize = "available"))]
    #[serde(serialize_with = "float_precission")]
    available_funds: f64,
    #[serde(rename(serialize = "held"))]
    #[serde(serialize_with = "float_precission")]
    held_funds: f64,
    #[serde(rename(serialize = "total"))]
    #[serde(serialize_with = "float_precission")]
    total_funds: f64,
    #[serde(rename(serialize = "locked"))]
    locked: bool,
    #[serde(skip_serializing)]
    transactions: Vec<Transaction>,
}

fn float_precission<S>(x: &f64, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_str(&format!("{:.4}", x))
}

fn read_input_file(path: &str) -> Result<Vec<Transaction>, Box<dyn Error>> {
    let mut txs = Vec::new();

    //trim all whitespace
    let mut rdr = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .flexible(true)
        .from_path(path)?;
    for result in rdr.deserialize() {
        // The iterator yields Result<StringRecord, Error>, so we check the
        // error here.
        let record: Transaction = result?;
        txs.push(record);
    }
    Ok(txs)
}

fn deposit(clients: &mut Vec<Client>, transaction: &Transaction) {
    match find_client(clients, transaction) {
        Some(mut c) => {
            c.available_funds += transaction.amount();
            c.total_funds += transaction.amount();
            c.transactions.push(transaction.clone());
        }
        None => clients.push(Client {
            client_id: transaction.client_id,
            available_funds: transaction.amount(),
            held_funds: 0.0,
            total_funds: transaction.amount(),
            locked: false,
            transactions: vec![transaction.clone()],
        }),
    }
}

fn find_client<'a>(
    clients: &'a mut Vec<Client>,
    transaction: &Transaction,
) -> Option<&'a mut Client> {
    clients
        .iter_mut()
        .find(|x| x.client_id == transaction.client_id && !x.locked)
}

fn withdrawal(clients: &mut Vec<Client>, transaction: &Transaction) {
    if let Some(mut c) = find_client(clients, transaction) {
        if c.available_funds >= transaction.amount() && c.total_funds >= transaction.amount() {
            c.available_funds -= transaction.amount();
            c.total_funds -= transaction.amount();
            c.transactions.push(transaction.clone());
        }
    }
}

fn dispute(clients: &mut Vec<Client>, transaction: &Transaction) {
    if let Some(client) = find_client(clients, transaction) {
        if let Some(t) = client
            .transactions
            .iter_mut()
            .find(|x| x.tx_id == transaction.tx_id)
        {
            client.available_funds -= t.amount();
            client.held_funds += t.amount();
            t.disputed = true;

            client.transactions.push(transaction.clone())
        }
    }
}

fn resolve(clients: &mut Vec<Client>, transaction: &Transaction) {
    if let Some(client) = find_client(clients, transaction) {
        if let Some(t) = client
            .transactions
            .iter_mut()
            .find(|x| x.tx_id == transaction.tx_id)
        {
            if t.disputed {
                client.held_funds -= t.amount();
                client.available_funds += t.amount();

                t.disputed = false;

                client.transactions.push(transaction.clone());
            }
        }
    }
}

fn chargeback(clients: &mut Vec<Client>, transaction: &Transaction) {
    if let Some(client) = find_client(clients, transaction) {
        if let Some(t) = client
            .transactions
            .iter_mut()
            .find(|x| x.tx_id == transaction.tx_id)
        {
            if t.disputed {
                if t.tx_type == "deposit" {
                    client.total_funds -= t.amount();
                    client.held_funds -= t.amount();
                } else {
                    client.total_funds -= -t.amount();
                    client.held_funds -= t.amount();
                }
                t.disputed = false;

                client.transactions.push(transaction.clone());
                client.locked = true;
            }
        }
    }
}

//Assumption: If a client does not exist in the "Database" of a Bank, the client can not withdraw any money from there.
//However, the bank will gladly accept the clients money and open up an account for the client.
//Clients only get added to the client vector if they added money before doing anything else.
fn handle_transactions(transactions: Vec<Transaction>) -> Vec<Client> {
    let mut clients: Vec<Client> = Vec::new();

    for t in transactions.iter() {
        match t.tx_type.as_str() {
            "deposit" => deposit(&mut clients, t),
            "withdrawal" => withdrawal(&mut clients, t),
            "dispute" => dispute(&mut clients, t),
            "resolve" => resolve(&mut clients, t),
            "chargeback" => chargeback(&mut clients, t),
            _ => continue,
        }
    }

    clients
}

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

    let clients = handle_transactions(txs);

    let mut wtr = csv::WriterBuilder::new().from_writer(std::io::stdout());

    for c in clients {
        wtr.serialize(c)?;
    }

    wtr.flush()?;

    Ok(())
}

#[test]
fn test_deposit() {
    let mut clients: Vec<Client> = Vec::new();
    let tx = Transaction {
        tx_type: "deposit".to_string(),
        client_id: 1,
        tx_id: 1,
        amount: Some(1.0),
        disputed: false,
    };

    deposit(&mut clients, &tx);

    assert_eq!(1.0, clients.first().unwrap().available_funds);
    assert_eq!(1.0, clients.first().unwrap().total_funds);

    deposit(&mut clients, &tx);
    assert_eq!(2.0, clients.first().unwrap().available_funds);
    assert_eq!(2.0, clients.first().unwrap().total_funds);
    assert_eq!(1, clients.len())
}

#[test]
fn test_withdrawal() {
    let mut clients: Vec<Client> = Vec::new();
    let tx_d = Transaction {
        tx_type: "deposit".to_string(),
        client_id: 1,
        tx_id: 1,
        amount: Some(1.0),
        disputed: false,
    };

    let mut tx_w = Transaction {
        tx_type: "withdrawal".to_string(),
        client_id: 1,
        tx_id: 2,
        amount: Some(0.5),
        disputed: false,
    };

    //use never funded the account, so there is no account
    withdrawal(&mut clients, &tx_w);
    assert!(clients.is_empty());

    //account is created, funded and takes out money
    deposit(&mut clients, &tx_d);
    withdrawal(&mut clients, &tx_w);

    assert_eq!(0.5, clients.first().unwrap().available_funds);
    assert_eq!(0.5, clients.first().unwrap().total_funds);

    //withdraw more money than the account has
    tx_w.amount = Some(5.0);

    withdrawal(&mut clients, &tx_w);

    assert_eq!(0.5, clients.first().unwrap().available_funds);
    assert_eq!(0.5, clients.first().unwrap().total_funds)
}

#[test]
fn test_withdrawal_resolve() {
    let mut clients: Vec<Client> = Vec::new();
    let tx_d = Transaction {
        tx_type: "deposit".to_string(),
        client_id: 1,
        tx_id: 1,
        amount: Some(10.0),
        disputed: false,
    };

    let tx_w = Transaction {
        tx_type: "withdrawal".to_string(),
        client_id: 1,
        tx_id: 2,
        amount: Some(2.0),
        disputed: false,
    };

    let tx_dispute = Transaction {
        tx_type: "dispute".to_string(),
        client_id: 1,
        tx_id: 2,
        amount: None,
        disputed: false,
    };

    let tx_resolve = Transaction {
        tx_type: "resolve".to_string(),
        client_id: 1,
        tx_id: 2,
        amount: None,
        disputed: false,
    };

    //create and fund the account
    deposit(&mut clients, &tx_d);

    //withdraw an amount
    withdrawal(&mut clients, &tx_w);

    //try to resolve a not disputed transaction (nothing should happen)
    resolve(&mut clients, &tx_resolve);
    {
        let c = clients.first().unwrap();

        assert_eq!(8.0, c.available_funds);
        assert_eq!(0.0, c.held_funds);
        assert!(!c.transactions.get(1).unwrap().disputed);
    }

    //dispute the withdrawal
    dispute(&mut clients, &tx_dispute);

    {
        let c = clients.first().unwrap();

        assert_eq!(6.0, c.available_funds);
        assert_eq!(2.0, c.held_funds);
        assert_eq!(8.0, c.total_funds);
        assert!(c.transactions.get(1).unwrap().disputed);
    }

    //resolve the disputed transaction
    resolve(&mut clients, &tx_resolve);

    let c = clients.first().unwrap();
    assert_eq!(8.0, c.available_funds);
    assert_eq!(0.0, c.held_funds);
    assert_eq!(8.0, c.total_funds);

    assert!(!c.transactions.get(1).unwrap().disputed);
    assert!(!c.locked)
}

#[test]
fn test_withdrawal_chargeback() {
    let mut clients: Vec<Client> = Vec::new();
    let tx_d = Transaction {
        tx_type: "deposit".to_string(),
        client_id: 1,
        tx_id: 1,
        amount: Some(10.0),
        disputed: false,
    };

    let tx_w = Transaction {
        tx_type: "withdrawal".to_string(),
        client_id: 1,
        tx_id: 2,
        amount: Some(2.0),
        disputed: false,
    };

    let tx_dispute = Transaction {
        tx_type: "dispute".to_string(),
        client_id: 1,
        tx_id: 2,
        amount: None,
        disputed: false,
    };

    let tx_chargeback = Transaction {
        tx_type: "chargeback".to_string(),
        client_id: 1,
        tx_id: 2,
        amount: None,
        disputed: false,
    };

    //create and fund the account
    deposit(&mut clients, &tx_d);

    //withdraw an amount
    withdrawal(&mut clients, &tx_w);

    //try to chargeback a not disputed transaction (nothing should happen)
    chargeback(&mut clients, &tx_chargeback);
    {
        let c = clients.first().unwrap();

        assert_eq!(8.0, c.available_funds);
        assert_eq!(0.0, c.held_funds);
        assert!(!c.transactions.get(1).unwrap().disputed);
    }

    //dispute the withdrawal
    dispute(&mut clients, &tx_dispute);

    //client reverses the transaction
    chargeback(&mut clients, &tx_chargeback);

    let c = clients.first().unwrap();
    assert_eq!(0.0, c.held_funds);
    assert_eq!(10.0, c.total_funds);
    assert!(c.locked)
}

#[test]
fn test_deposit_chargeback() {
    let mut clients: Vec<Client> = Vec::new();
    let tx_d = Transaction {
        tx_type: "deposit".to_string(),
        client_id: 1,
        tx_id: 1,
        amount: Some(10.0),
        disputed: false,
    };

    let tx_dd = Transaction {
        tx_type: "deposit".to_string(),
        client_id: 1,
        tx_id: 2,
        amount: Some(10.0),
        disputed: false,
    };

    let tx_dispute = Transaction {
        tx_type: "dispute".to_string(),
        client_id: 1,
        tx_id: 2,
        amount: None,
        disputed: false,
    };

    let tx_chargeback = Transaction {
        tx_type: "chargeback".to_string(),
        client_id: 1,
        tx_id: 2,
        amount: None,
        disputed: false,
    };

    deposit(&mut clients, &tx_d);
    deposit(&mut clients, &tx_dd);

    assert_eq!(20.0, clients.first().unwrap().total_funds);

    dispute(&mut clients, &tx_dispute);
    chargeback(&mut clients, &tx_chargeback);

    assert_eq!(1, clients.len());
    {
        let c = clients.first().unwrap();

        assert_eq!(10.0, c.total_funds);
        assert!(c.locked)
    }

    //doing something with a locked account is not possible
    deposit(&mut clients, &tx_dd);
    assert_eq!(10.0, clients.first().unwrap().total_funds)
}


#[test]
fn test_deposit_resolve() {
    let mut clients: Vec<Client> = Vec::new();
    let tx_d = Transaction {
        tx_type: "deposit".to_string(),
        client_id: 1,
        tx_id: 1,
        amount: Some(10.0),
        disputed: false,
    };

    let tx_dd = Transaction {
        tx_type: "deposit".to_string(),
        client_id: 1,
        tx_id: 2,
        amount: Some(10.0),
        disputed: false,
    };

    let tx_dispute = Transaction {
        tx_type: "dispute".to_string(),
        client_id: 1,
        tx_id: 2,
        amount: None,
        disputed: false,
    };

    let tx_chargeback = Transaction {
        tx_type: "resolve".to_string(),
        client_id: 1,
        tx_id: 2,
        amount: None,
        disputed: false,
    };

    deposit(&mut clients, &tx_d);
    deposit(&mut clients, &tx_dd);

    assert_eq!(20.0, clients.first().unwrap().total_funds);

    dispute(&mut clients, &tx_dispute);
    resolve(&mut clients, &tx_chargeback);

    assert_eq!(1, clients.len());
    let c = clients.first().unwrap();

    assert_eq!(0.0, c.held_funds);
    assert_eq!(20.0, c.total_funds);
}
