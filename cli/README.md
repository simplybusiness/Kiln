# Kiln CLI
This is the CLI tool that users and CI servers will use to execute scans with Kiln. It is responsible for pulling and using tool containers.

## Configuring
### Required configuration
The Kiln CLI requires that you configure the name of the application you are scanning and the endpoint to send scan results to. This is configured using a file called `kiln.toml` in your project root. For a primer on the TOML format, see [https://github.com/toml-lang/toml](https://github.com/toml-lang/toml).

#### Example minimal `kiln.toml`

``` toml
app_name="My Application"
data_collector_url="https://kiln.my-domain.com"
```

### Optional configuration
#### Docker Registry Credentials
By default, Kiln will interact with the Docker registry as an anonymous client, which is fine for testing and prototyping. Due to the changes to Rate Limits at Docker Hub, you are strongly encouraged to provide credentials to Kiln for production use to avoid encountering a rate limit error. Release builds of Kiln try to be as conservative as possible with your Docker Hub rate limit allowance by not repulling images if a suitable one exists, but by providing credentials for the CLI to use, you are less likely to encounter issues.

Currently, Kiln only supports providing credentials in a pair of environment variables: `KILN_DOCKER_USERNAME` and `KILN_DOCKER_PASSWORD`. Support for better secret storage options is planned for a future release, and is being tracked here: https://github.com/simplybusiness/Kiln/issues/272. Although the environment variable name references a password, the `KILN_DOCKER_PASSWORD` environment variable can also be used to supply Docker Hub Personal Access Tokens for accounts with Two Factor Authentication enabled, as well as the Base-64 encoded JWTs used by AWS Elastic Container Registry.

#### Custom Docker Registries
In addition to using the default tool images hosted on Docker Hub, you can also configure the Kiln CLI to pull tool images from your own Docker registry. To do this, set the `KILN_DOCKER_REGISTRY` environment variable to the URL for the registry and repo that contains the tool images you wish to use.

#### Custom Tool image names and tags
The Kiln CLI can also be configured to use a custom tool image name and tag. You can do this with the `--tool-image-name` command line switch and provide the tool name and optionally the tag as a colon separated value. If you ommit the tag, the CLI will default to using it's version number as the tag (or `git-latest` when running a debug build).

#### Suppressing issues
If Kiln is reporting issues that you determine are false positives or would otherwise like to suppress from future alerts, you can do that by adding the finding to the project's `kiln.toml` configuration file. An example is included below:
```toml
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

All fields are requires, except the expiry date. A suppression expiry date should only be omitted for issues that can't ever be fixed or are a false positive. The issue_hash for a particular issue can be found from the alert sent to Slack by the Slack-connector (if deployed), or calculated using the instructions in the [Issue Hash Format](#issue-hash-format) section below.

##### Issue Hash Format
The Issue Hash format used for dependency events is described in the following runnable Rust Playground, which you can also use to calculate an Issue Hash manually if you don't have the Slack-connector deployed: https://play.rust-lang.org/?version=stable&mode=debug&edition=2018&gist=7269ca98533cf0059dc0e3e192b25d00

## Building
First, ensure you have cargo-make installed by running cargo install cargo-make. Then from this directory, run `cargo make build-kiln-cli`. This will build the CLI using your current Rust toolchain. It will then copy it to the `bin` directory in the root of the Kiln repo.

