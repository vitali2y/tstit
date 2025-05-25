### tstit - Test It. REST It.

**Lightweight and blazing fast CLI tool for automated REST API testing.**


#### Usage
```
➜  export URL="http://127.0.0.1:8081"; export TKN=...
➜  cargo r ./tests/ccs/customer/
~...~
     Running `target/debug/tstit ./tests/ccs/customer/`
tstit v0.2.0 - Test It. REST It.
 INFO  tstit > found 3 testplans
 INFO  tstit > processing ./tests/ccs/customer/customer_create.toml...
 ERROR tstit > testplan failed: API Error 17: "db error: failed to create customer: UNIQUE constraint failed: customer.contactemail"
 INFO  tstit > processing ./tests/ccs/customer/customer_patch.toml...
 INFO  tstit::engine > validation successful
 INFO  tstit         > testplan succeeded
 INFO  tstit         > processing ./tests/ccs/customer/customer_get.toml...
 INFO  tstit::engine > validation successful
 INFO  tstit         > testplan succeeded
 INFO  tstit         > test execution completed, success: 2, failed: 1
➜  cat ./tests/ccs/customer/customer_get.toml
[in]
url = "/v1/customer/5882323"

[out.expect]
firstname = "John"
lastname = "Dow"
phone = "8024633333"
➜
```
