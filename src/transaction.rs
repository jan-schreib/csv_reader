use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::Deserialize;

use crate::client::Client;

#[derive(Deserialize, Debug, Clone)]
pub struct Transaction {
    #[serde(rename(deserialize = "type"))]
    pub tx_type: String,
    #[serde(rename(deserialize = "client"))]
    pub client_id: u16,
    #[serde(rename(deserialize = "tx"))]
    pub tx_id: u32,
    pub amount: Option<Decimal>,
    #[serde(skip_deserializing)]
    pub disputed: bool,
}

impl Transaction {
    // Assumption: If a client does not exist in the "Database" of a Bank, the client can not withdraw any money from there.
    // However, the bank will gladly accept the clients money and open up an account for the client.
    // Clients only get added to the client vector if they added money before doing anything else.

    /// This function handles transactions and returns an vector of users.
    /// Depending on the transaction type a different method for handling the transaction will be used.
    /// If there are no valid transactions an empty vector will be returned.
    ///
    /// # Example
    ///
    /// ```
    /// let tx_d = Transaction {
    ///     tx_type: "deposit".to_string(),
    ///     client_id: 1,
    ///     tx_id: 1,
    ///     amount: Some(dec!(10.0)),
    ///     disputed: false,
    /// };
    ///
    /// let tx_w = Transaction {
    ///     tx_type: "withdrawal".to_string(),
    ///     client_id: 1,
    ///     tx_id: 2,
    ///     amount: Some(dec!(2.0)),
    ///     disputed: false,
    /// };
    ///
    /// let txs = vec![tx_d, tx_w];
    ///
    /// let clients = handle_transactions(txs)
    ///
    /// ```
    pub fn handle_transactions(transactions: Vec<Transaction>) -> Vec<Client> {
        let mut clients: Vec<Client> = Vec::new();

        for t in transactions.iter() {
            match t.tx_type.as_str() {
                "deposit" => t.deposit(&mut clients),
                "withdrawal" => t.withdrawal(&mut clients),
                "dispute" => t.dispute(&mut clients),
                "resolve" => t.resolve(&mut clients),
                "chargeback" => t.chargeback(&mut clients),
                _ => {
                    eprintln!("Invalid transaction: {:?}", t);
                    continue;
                }
            }
        }

        clients
    }

    fn amount(&self) -> Decimal {
        self.amount.unwrap_or_else(|| dec!(0.0))
    }

    /// Helper function to find the corresponding unlocked client to a transaction.
    fn find_client<'a>(&self, clients: &'a mut Vec<Client>) -> Option<&'a mut Client> {
        clients
            .iter_mut()
            .find(|x| x.client_id == self.client_id && !x.locked)
    }

    /// This is the only function that is allowed to create users.
    /// When there is a "deposit" transaction and the user is not found, the user will be created.
    /// If the user is found and is unlocked, the funds will be added.
    fn deposit(&self, clients: &mut Vec<Client>) {
        match clients.iter_mut().find(|x| x.client_id == self.client_id) {
            Some(c) => {
                if !c.locked {
                    c.available_funds += self.amount();
                    c.total_funds += self.amount();
                    c.transactions.push(self.clone());
                }
            }
            None => clients.push(Client {
                client_id: self.client_id,
                available_funds: self.amount(),
                held_funds: Decimal::new(0, 0),
                total_funds: self.amount(),
                locked: false,
                transactions: vec![self.clone()],
            }),
        }
    }

    /// Handles withdrawals if and only if the user exists and the money in the account is more or equal to the
    /// amount of the withdrawal. No margin allowed.
    fn withdrawal(&self, clients: &mut Vec<Client>) {
        if let Some(c) = self.find_client(clients) {
            if c.available_funds >= self.amount() && c.total_funds >= self.amount() {
                c.available_funds -= self.amount();
                c.total_funds -= self.amount();
                c.transactions.push(self.clone());
            }
        }
    }

    /// The clients way to claim that an transaction was errorneous.
    /// Disputed transactions will be handled via resolving the issue or a chargeback by the client.
    /// Funds of the transaction in question will be held (held_funds) and are not available to the client
    /// until the situation is resolved. This function also marks the transaction in question as disputed.
    fn dispute(&self, clients: &mut Vec<Client>) {
        if let Some(client) = self.find_client(clients) {
            if let Some(t) = client
                .transactions
                .iter_mut()
                .find(|x| x.tx_id == self.tx_id)
            {
                client.available_funds -= t.amount();
                client.held_funds += t.amount();
                t.disputed = true;

                client.transactions.push(self.clone())
            }
        }
    }

    /// A disputed transaction gets resolved and the held funds will be given back and are again usable
    /// for the client. If the transaction is not marked as disputed, the function call will be ignored.
    fn resolve(&self, clients: &mut Vec<Client>) {
        if let Some(client) = self.find_client(clients) {
            if let Some(t) = client
                .transactions
                .iter_mut()
                .find(|x| x.tx_id == self.tx_id)
            {
                if t.disputed {
                    client.held_funds -= t.amount();
                    client.available_funds += t.amount();

                    t.disputed = false;

                    client.transactions.push(self.clone());
                }
            }
        }
    }

    /// Client reverses a transaction. This will immediately freeze (lock) the client.
    /// A locked client can not do any more transactions.
    /// The held funds and total funds will be reduced by the disputed amount.
    /// If the disputed transaction was a withdrawal, the funds will be re-added to the account
    /// for any other transaction type the held funds and total funds will be reduced.
    /// If the transaction is not marked as disputed, the function call will be ignored.
    fn chargeback(&self, clients: &mut Vec<Client>) {
        if let Some(client) = self.find_client(clients) {
            if let Some(t) = client
                .transactions
                .iter_mut()
                .find(|x| x.tx_id == self.tx_id)
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

                    client.transactions.push(self.clone());
                    client.locked = true;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Client;
    use super::Transaction;
    use rust_decimal_macros::dec;

    #[test]
    fn test_deposit() {
        let mut clients: Vec<Client> = Vec::new();
        let tx = Transaction {
            tx_type: "deposit".to_string(),
            client_id: 1,
            tx_id: 1,
            amount: Some(dec!(1.0)),
            disputed: false,
        };

        tx.deposit(&mut clients);

        assert_eq!(dec!(1.0), clients.first().unwrap().available_funds);
        assert_eq!(dec!(1.0), clients.first().unwrap().total_funds);

        tx.deposit(&mut clients);
        assert_eq!(dec!(2.0), clients.first().unwrap().available_funds);
        assert_eq!(dec!(2.0), clients.first().unwrap().total_funds);
        assert_eq!(1, clients.len())
    }

    #[test]
    fn test_withdrawal() {
        let mut clients: Vec<Client> = Vec::new();
        let tx_d = Transaction {
            tx_type: "deposit".to_string(),
            client_id: 1,
            tx_id: 1,
            amount: Some(dec!(1.0)),
            disputed: false,
        };

        let mut tx_w = Transaction {
            tx_type: "withdrawal".to_string(),
            client_id: 1,
            tx_id: 2,
            amount: Some(dec!(0.5)),
            disputed: false,
        };

        //use never funded the account, so there is no account
        tx_w.withdrawal(&mut clients);
        assert!(clients.is_empty());

        //account is created, funded and takes out money
        tx_d.deposit(&mut clients);
        tx_w.withdrawal(&mut clients);

        assert_eq!(dec!(0.5), clients.first().unwrap().available_funds);
        assert_eq!(dec!(0.5), clients.first().unwrap().total_funds);

        //withdraw more money than the account has
        tx_w.amount = Some(dec!(5.0));

        tx_w.withdrawal(&mut clients);

        assert_eq!(dec!(0.5), clients.first().unwrap().available_funds);
        assert_eq!(dec!(0.5), clients.first().unwrap().total_funds)
    }

    #[test]
    fn test_withdrawal_resolve() {
        let mut clients: Vec<Client> = Vec::new();
        let tx_d = Transaction {
            tx_type: "deposit".to_string(),
            client_id: 1,
            tx_id: 1,
            amount: Some(dec!(10.0)),
            disputed: false,
        };

        let tx_w = Transaction {
            tx_type: "withdrawal".to_string(),
            client_id: 1,
            tx_id: 2,
            amount: Some(dec!(2.0)),
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
        tx_d.deposit(&mut clients);

        //withdraw an amount
        tx_w.withdrawal(&mut clients);

        //try to resolve a not disputed transaction (nothing should happen)
        tx_resolve.resolve(&mut clients);
        {
            let c = clients.first().unwrap();

            assert_eq!(dec!(8.0), c.available_funds);
            assert_eq!(dec!(0.0), c.held_funds);
            assert!(!c.transactions.get(1).unwrap().disputed);
        }

        //dispute the withdrawal
        tx_dispute.dispute(&mut clients);

        {
            let c = clients.first().unwrap();

            assert_eq!(dec!(6.0), c.available_funds);
            assert_eq!(dec!(2.0), c.held_funds);
            assert_eq!(dec!(8.0), c.total_funds);
            assert!(c.transactions.get(1).unwrap().disputed);
        }

        //resolve the disputed transaction
        tx_resolve.resolve(&mut clients);

        let c = clients.first().unwrap();
        assert_eq!(dec!(8.0), c.available_funds);
        assert_eq!(dec!(0.0), c.held_funds);
        assert_eq!(dec!(8.0), c.total_funds);

        assert!(!c.transactions.get(1).unwrap().disputed);
        assert!(!c.locked)
    }
    #[cfg(test)]
    #[test]
    fn test_withdrawal_chargeback() {
        let mut clients: Vec<Client> = Vec::new();
        let tx_d = Transaction {
            tx_type: "deposit".to_string(),
            client_id: 1,
            tx_id: 1,
            amount: Some(dec!(10.0)),
            disputed: false,
        };

        let tx_w = Transaction {
            tx_type: "withdrawal".to_string(),
            client_id: 1,
            tx_id: 2,
            amount: Some(dec!(2.0)),
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
        tx_d.deposit(&mut clients);

        //withdraw an amount
        tx_w.withdrawal(&mut clients);

        //try to chargeback a not disputed transaction (nothing should happen)
        tx_chargeback.chargeback(&mut clients);
        {
            let c = clients.first().unwrap();

            assert_eq!(dec!(8.0), c.available_funds);
            assert_eq!(dec!(0.0), c.held_funds);
            assert!(!c.transactions.get(1).unwrap().disputed);
        }

        //dispute the withdrawal
        tx_dispute.dispute(&mut clients);

        //client reverses the transaction
        tx_chargeback.chargeback(&mut clients);

        let c = clients.first().unwrap();
        assert_eq!(dec!(0.0), c.held_funds);
        assert_eq!(dec!(10.0), c.total_funds);
        assert!(c.locked)
    }

    #[test]
    fn test_deposit_chargeback() {
        let mut clients: Vec<Client> = Vec::new();
        let tx_d = Transaction {
            tx_type: "deposit".to_string(),
            client_id: 1,
            tx_id: 1,
            amount: Some(dec!(10.0)),
            disputed: false,
        };

        let tx_dd = Transaction {
            tx_type: "deposit".to_string(),
            client_id: 1,
            tx_id: 2,
            amount: Some(dec!(10.0)),
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

        tx_d.deposit(&mut clients);
        tx_dd.deposit(&mut clients);

        assert_eq!(dec!(20.0), clients.first().unwrap().total_funds);

        tx_dispute.dispute(&mut clients);
        tx_chargeback.chargeback(&mut clients);

        assert_eq!(1, clients.len());
        {
            let c = clients.first().unwrap();

            assert_eq!(dec!(10.0), c.total_funds);
            assert!(c.locked)
        }

        //doing something with a locked account is not possible
        tx_dd.deposit(&mut clients);
        assert_eq!(dec!(10.0), clients.first().unwrap().total_funds)
    }

    #[test]
    fn test_deposit_resolve() {
        let mut clients: Vec<Client> = Vec::new();
        let tx_d = Transaction {
            tx_type: "deposit".to_string(),
            client_id: 1,
            tx_id: 1,
            amount: Some(dec!(10.0)),
            disputed: false,
        };

        let tx_dd = Transaction {
            tx_type: "deposit".to_string(),
            client_id: 1,
            tx_id: 2,
            amount: Some(dec!(10.0)),
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

        tx_d.deposit(&mut clients);
        tx_dd.deposit(&mut clients);

        assert_eq!(dec!(20.0), clients.first().unwrap().total_funds);

        tx_dispute.dispute(&mut clients);
        tx_chargeback.resolve(&mut clients);

        assert_eq!(1, clients.len());
        let c = clients.first().unwrap();

        assert_eq!(dec!(0.0), c.held_funds);
        assert_eq!(dec!(20.0), c.total_funds);
    }
}
