A tool to read a csv.

Usage:

```bash
$ ./csv_read input.csv
```

Assumptions that where made:

1. If a client does not exist in the "database" of a bank, the client can not withdraw any money from there.
   However, the bank will gladly accept the clients money and open up an account for the client. Clients only get added to the client vector if they added money before doing anything else.

2. Transactions on locked accounts have no effect and will be ignored. 

For handling monetary values the rust_decimal crate is used. The decision was based on the fact that f64 can have round-off errors and the crate enables
integral and fractional calculations. 