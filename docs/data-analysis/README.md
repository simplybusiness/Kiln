# Kiln Data Analysis 

This document aims to help you understand how to analyse the data being gathered by Kiln with a practical example using Jupyterhub and Python. The question we will be trying to answer is this: "What is the average time to remediate a vulnerability in an open source dependency of a given project?". We'll be generating our test data set by running Kiln on every commit to the `master` branch of three open source Ruby projects: [OWASP RailsGoat](https://github.com/OWASP/railsgoat), [Mastodon](https://github.com/tootsuite/mastodon) & [GitLab](https://gitlab.com/gitlab-org/gitlab). 

## Prerequisites

* A Kiln stack - A dedicated stack is recommended, follow instructions at [docs/quickstart/README.md](docs/quickstart/README.md) to setup a dedicated cluster for testing, but do not deploy the Slack-connector.
* Git
* Python 3 - Instructions for setup can be found here: [https://wiki.python.org/moin/BeginnersGuide/Download](https://wiki.python.org/moin/BeginnersGuide/Download)
