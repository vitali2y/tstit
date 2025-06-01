### tstit - Test It. REST It.

**Lightweight and blazing fast CLI tool for automated REST API testing.**


#### Usage
On first console just run your backend app, or [fake_server](examples/fake_server.rs):
```
➜  export TSTIT_URL="http://127.0.0.1:8081"; export TSTIT_TKN=...
➜  cargo r --example fake_server
~...~
fake_server is running @ http://127.0.0.1:8081
~...~
```

On second console just run your tests, or [tests for fake_server](tests/customer/):
```
➜  export TSTIT_URL="http://127.0.0.1:8081"; export TSTIT_TKN=...
➜  cargo r ./tests/customer/   
~...~
tstit v0.3.0 - Test It. REST It.
 INFO  tstit > found 4 testplans
 INFO  tstit > processing ./tests/customer/10customer_create.toml...
actual: 2, expected: 0
 INFO  tstit::engine > validation successful
 INFO  tstit::engine > assigned 2 to $TSTIT_ID var
 INFO  tstit         > testplan succeeded
 INFO  tstit         > processing ./tests/customer/20customer_get.toml...
 INFO  tstit::engine > validation successful
 INFO  tstit         > testplan succeeded
 INFO  tstit         > processing ./tests/customer/30customer_patch.toml...
actual: 2, expected: 0
 INFO  tstit::engine > validation successful
 INFO  tstit         > testplan succeeded
 INFO  tstit         > processing ./tests/customer/40customer_get.toml...
 INFO  tstit::engine > validation successful
 INFO  tstit         > testplan succeeded
 INFO  tstit         > test execution completed, success: 4, failed: 0
➜  ./target/debug/tstit --help
Usage: tstit [<paths...>] [-v] [-V]

tstit - Test It. REST It.

Positional Arguments:
  paths             path(s) to testplan TOML files or directories containing
                    testplans

Options:
  -v, --verbose     enable verbose output
  -V, --version     print version information
  --help, help      display usage information
➜
```
