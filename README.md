# transacto
Simple transaction software exercise

## Assumptions
- A dispute can only happen to a deposit. We cannot hold funds that were withdrawn as that would "create money" and would open the door for double spending. It's assumed a dispute on a withdrawn would happen on the other client's "deposit".
- Disputes are final, once a resolution has been reached. An appeal to a dispute would perhaps make sense. This would likely need further human intervention, so for simplicity they are final (changing this rule would also be trivial).
- Accounts can temporarily go negative due to a dispute on an early transaction, if the client no longer has enough money. In the real world this would have to raise a flag, which would then possibly set a debt on the client, and potentially require human intervention. Locking an account in this situation could make sense but, practically speaking, having negative balance effectively makes it behave the same way, so this was left out. As a clarification, withdrawals can never set the balance to negative. A negative account can still receive deposits, as that can be used to "pay the debt".
- All transactions are idempotent. If a transaction id is repeated that second transaction is ignored. This helps if the code is put in a distributed system where retries will likely be necessary and might result in messages being recived more than once, for example, due to the [two generals problem](https://en.wikipedia.org/wiki/Two_Generals%27_Problem).
- Locked accounts can no longer accept withdrawals. Deposits and disputes are accepted though.
- All transaction records contain all columns. For example a `Dispute` will still contain the `amount` column, albeit empty. A `Deposit` or `Withdrawal` with an empty, negative or 0 amount will, however, be ignored.

## Design decisions
The system takes advantage of the type system to ensure correctness. The transactions are parsed into concrete data types (`Deposit`, `Withdrawal`, `Dispute`, `Resolve` and `Chargeback`) and implement the trait `ExecutableTransaction`. The trait contains the functions `execute`, `dispute`, `resolve` and `chargeback`, which are implemented accordingly by each transaction type. This makes it easy to add new transactions as well as easily add dispute functionality when needed. E.g., if we decide later that `Withdrawal` can indeed be disputed, we'd just need to change the `dispute`, `resolve` and `chargeback` functions.

The `Ledger` holds the clients' data as well as the history of transactions. Transactions need to be stored so that they can be disputed (and it's also probably a good idea for record keeping). Because `traits` can't be stored in data structures on their own, a decision had to be made here. There were several options for this:
- Since we only dispute `Deposits`, technically only this data type needs to be stored. This would not be very scalable though and any changes to the requirements later would throw this solution out of the window (e.g. a new transaction supports disputes).
- We store it as a `Box<dyn ExecutableTransaction>`. This maintains the pros of using the `trait`, compile-time safety, polymorphism and extensibility. However, performance suffers as it adds a runtime overhead.
- Using `enum` instead of `trait`. This would reduce the runtime overhead however extending the transactions becomes a bit more cumbersome as we need to add checks for the type on every function.
- Keeping the `trait` but nesting the concrete types in an `enum` with the help of `enum_dispatch`.
In the end the `enum_dispatch` option was chosen as this gave the flexibility of the `trait` and the pros listed above, while not needing to keep memory in the heap with `Box`. This means adding a new transaction requires a new element in the enum but the crate handles everything else.

Since TCP connections was a consideration all transactions are idempotent. Since transaction ids are globally unique, any transaction with an id that has been used will be discarded.

A simple `event_logger` is used for debugging. Since the output must only contain the csv data of the clients, all non critical errors are logged as debug. This also makes it easy if later on logging to a file or streaming it to a separate logger becomes a requirement, for example. For the same reason, custom errors were created so that it is easy to programmatically check what failed in a transaction, if required.

A `rust-toolchain` file with the `channel` set to `1.79.0` was introduced mainly because of [this issue](https://github.com/rust-lang/rust-analyzer/issues/17662) with `rust-analyzer` in `VSCode`.

## Other considerations
If the code needs to be part of a Server it would be wise to make the processing of the csv data asynchronous. This is not done in this version but would be not complicated to modify. For example, the crate `csv_async` along with `tokio` could be used to help with this. It's important that the `Ledger` is not edited concurrently, however, as the operations are not thread safe. For that, at the very least an `Arc` would be necessary. One simple implementation to make it more async would be to spawn a task to execute the transactions with the `Ledger` and another to process the csv records with `csv_async`. A `tokio channel` could be used to send the processed record to the `Ledger` task to be executed. This way if a transaction takes longer, the program can continue reading the csv file, for example. These modifications would not require changes to the core code but only the "glue" like the data module.

It is not possible to use the `Ledger` to view a record of all transactions in chronological order. It is also not easy to see all transactions from a specific client only (unless we iterate all anyway). Further, the `dispute` and its family of transactions are not recorded due to them not having a unique id of their own. These are likely fair requirements for a system deployed in the real world. A new recording strategy would need to be implemented to support these features. Having said that, a separate module, that gets fed the transactions as they are processed, could be used for recording purposes only. This way we'd separate functionality and keep transacto simple.

As a final thought, in a situation where multiple TCP connections are streaming large csv files, we could consider partially flushing the data out before finishing, since if there are many clients (and many transactions) what's kept in memory could drastically increase. This would need careful consideration though, since we probably still need access to the data, which might mean pulling it out again from less volatile memory, potentially causing a hit on performance.
