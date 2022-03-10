# HTTP load testing tool (experiment)
> Do not use this program for bad purposes, this may lead to criminal liability!
# USAGE

## How to build a program
```
# cargo build --release
# ./target/release/denial-of-service --help
```

## How to start a program
```
# ./target/release/denial-of-service -u https://example.com -fpe
```

## About flags
```
-e, --error-mode    Do not display errors
-f, --force         Start DoS without website status checking
-h, --help          Print help information
-p, --proxy         Needs to use proxy servers
-u, --url <URL>     Website URL to attack
```
