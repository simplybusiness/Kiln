# Kiln CLI
This is the CLI tool that users and CI servers will use to interact with Kiln. It is responsible for pulling and using tool containers.

## Building
First, ensure you have cargo-make installed by running cargo install cargo-make. Then from this directory, run `cargo make build-kiln-cli`. This will build the CLI using your current Rust toolchain. It will then copy it to the `bin` directory in the root of the Kiln repo.

## Configuring
The Kiln CLI expects a `kiln.toml` file in the root directory of your project. For a primer on the TOML format, see [https://github.com/toml-lang/toml](https://github.com/toml-lang/toml).

### Example `kiln.toml`

``` toml
app_name="My Application"
data_collector_url="https://kiln.my-domain.com"

[[suppressed_issues]]
issue_hash="42dad938ec93cafda2461a9281753376bcd36210a526b198d46682ac9b5d789f"
suppression_reason="CVE 2020-12345 in some_package. Fix not available from upstream yet."
expiry_date="2020-05-22T15:00:00+00:00"
suppressed_by="Dan Murphy"

[[suppressed_issues]]
issue_hash="aa1b9e3f6acb519e3b2cba05cbbafb9318e47ff33ee1225f23d394af992d347a"
suppression_reason="CVE 2020-12346 in some_other_package. Fix not available from upstream yet."
expiry_date="2020-05-22T15:00:00+00:00"
suppressed_by="Dan Murphy"
```

Only two of the fields in `kiln.toml` are requires: `app_name` and `data_collector_url`. If you need to suppress issues, only the expiry date is an optional field, and should only be omitted for issues that can't ever be fixed or are a false positive. The issue_hash for a particular finding can be found from the alert sent to Slack by the Slack-connector.
