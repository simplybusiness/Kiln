# Kiln Data Analysis 

This document aims to help you understand how to analyse the data being gathered by Kiln with a practical example using Jupyterhub and Python. The question we will be trying to answer is this: "What is the average time to remediate a vulnerability in an open source dependency of a given project?". We'll be generating our test data set by running Kiln on every commit to the `master` branch of three open source Ruby projects: [OWASP RailsGoat](https://github.com/OWASP/railsgoat), [Mastodon](https://github.com/tootsuite/mastodon) & [GitLab](https://gitlab.com/gitlab-org/gitlab). 

Jupyterhub is a browser-based, multi-user interactive computing environment that supports dozens of languages, which makes it ideal for exploratory data analysis. By deploying Jupyterhub to the Kubernetes cluster we're hosting, we will have access to an environment with plenty of computing resources, available on-demand for performing exploration of the events stored in Kafka.

## Prerequisites

* A Kiln stack - A dedicated stack is recommended, follow instructions at [..//quickstart/README.md](../quickstart/README.md) to setup a dedicated cluster for testing, but do not deploy the Slack-connector. This document assumes you have followed those instructions and have the listed prerequisites installed.
* Git
* Python 3 - Instructions for setup can be found here: [https://wiki.python.org/moin/BeginnersGuide/Download](https://wiki.python.org/moin/BeginnersGuide/Download)

## Provisioning instances for analysis

Because the test data we will be generating will result in around 1 million events to analyse, we need to provision an additional instance group for our Kubernetes cluster to host Jupyter notebook instances for users.

**WARNING: The instance type provisioned below has a significantly higher cost per hour than the t3a.medium instances suggested for the rest of the cluster. When you aren't using them, be sure to follow the instructions to scale this group down to zero instances to avoid being charged for resources you aren't using.** As an illustration of the cost involved, the 6 `t3a.medium` on-demand instances used for the rest of the cluster cost $6.12 per day to run in eu-west-2. The single `t3a.2xlarge` on-demand instance used for analysis costs $8.16 per day to run in eu-west-2.


Creating the instance group:
``` shell
kops create ig himemnodes --subnet eu-west-2a
```

This will open your console editor, add the following lines then save and quit. If you want additional analysis nodes, you can change the `minSize` and `maxSize` values accordingly.

``` yaml
maxSize: 1
minSize: 1
machineType: t3a.2xlarge
nodeLabels:
  hub.jupyter.org/node-purpose: user
taints:
  - hub.jupyter.org/dedicated=user:NoSchedule
```
Then apply the changes by running the following:

``` shell
kops update cluster ${NAME} #This will show a preview of the changes to be applied
kops update cluster ${NAME} --yes #This will apply the changes
```

When you aren't using the analysis instances, you can scale them down to zero (without losing data!) by running the following:
``` shell
kops edit ig himemnodes
```

This will open an editor containing the YAML definition for the instance group. Change the `minSize` and `maxSize` values to 0, save and quit. Then apply the changes by running the following:

``` shell
kops update cluster ${NAME} #This will show a preview of the changes to be applied
kops update cluster ${NAME} --yes #This will apply the changes
kops rolling-update cluster ${NAME} #This will trigger the instance group scale down
```

## Deploying JupyterHub

We will be deploying JupyterHub using their official Helm chart, using a values YAML file to customise the deployment to configure the docker image to use for user environments, an authentication mechanism etc.

### Requesting an AWS ACM certificate

### Generating a random token to secure proxy and hub communications

### Configuring OAuth authentication

### Deploying JupyterHub

## Generating Test Data

## Performing Data Analysis
