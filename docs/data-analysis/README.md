# Kiln Data Analysis 

This document aims to help you understand how to analyse the data being gathered by Kiln with a practical example using Jupyterhub and Python. The question we will be trying to answer is this: "What is the average time to remediate a vulnerability in an open source dependency of a given project?". We'll be generating our test data set by running Kiln on every commit to the `master` branch of three open source Ruby projects: [OWASP RailsGoat](https://github.com/OWASP/railsgoat), [Mastodon](https://github.com/tootsuite/mastodon) & [GitLab](https://gitlab.com/gitlab-org/gitlab). 

Jupyterhub is a browser-based, multi-user interactive computing environment that supports dozens of languages, which makes it ideal for exploratory data analysis. By deploying Jupyterhub to the Kubernetes cluster we're hosting, we will have access to an environment with plenty of computing resources, available on-demand for performing exploration of the events stored in Kafka.

## Prerequisites

* A Kiln stack - A dedicated stack is recommended, follow instructions at [docs/quickstart/README.md](../quickstart/README.md) to setup a dedicated cluster for testing, but do not deploy the Slack-connector. This document assumes you have followed those instructions and have the listed prerequisites installed.
* Git
* Python 3 - Instructions for setup can be found here: [https://wiki.python.org/moin/BeginnersGuide/Download](https://wiki.python.org/moin/BeginnersGuide/Download)
* Pipenv - Instructions for setup can be found here: [https://github.com/pypa/pipenv](https://github.com/pypa/pipenv)

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

We're going to be using kiln-jupyterhub.mysubdomain.mydomain.tld (replacing placeholders as appropriate) as the DNS name for hosting JupyterHub and will be serving it over TLS. AWS ACM provides free, auto-renewing TLS certificates that are publically trusted, follow the documentation that AWS provide, which can be found at: [https://docs.aws.amazon.com/acm/latest/userguide/gs-acm-request-public.html](https://docs.aws.amazon.com/acm/latest/userguide/gs-acm-request-public.html). Be sure to create the ACM certificate in the same region that the cluster is deployed to (in this instance, eu-west-2). When prompted for a domain, use "kiln-jupyterhub.my-subdomain.mydomain.tld", replacing the subdomain and domain with the appropriate values for your deployment.

Once your ACM certificate has been issued, take a note of it's ARN and then replace the example ARN in jupyterhub-values.yml with the ARN for your new certificate. The key for this value is `proxy->service->snnotations->service.beta.kubernetes.io/aws-load-balancer-ssl-cert`. In the same set of annotations, you will also need to fill in the correct value for the external DNS record you want to be created for Jupyterhub.

### Generating a random token to secure proxy and hub communications

The JupyterHub Proxy and Hub components secure their communications using a 32 byte random token. You can generate this securely using OpenSSL: `openssl rand -hex 32`. The 32 byte hex string that is produced should be copied into the jupyterhub-values.yml file as the value for the key `proxy->secretToken`.

### Configuring OAuth authentication

By default, JupyterHub deploys with a dummy authentication module active, which will accept any username and password. As this service will be public facing, we want to ensure only authorized users can login and use the JupyterHub cluster. JupyterHub can be configured to use OAuth 2 delegated authentication using a number of providers, including GitHub and Google. The jupyterhub-values.yml file has the configuration stub required for GitHub authentication, but this could be replaced with another service if you do not use GitHub. Follow the JupyterHub Authentication guide to configure your authentication service of choice, which can be found here: [https://zero-to-jupyterhub.readthedocs.io/en/latest/administrator/authentication.html](https://zero-to-jupyterhub.readthedocs.io/en/latest/administrator/authentication.html).

### Deploying JupyterHub

Now that you have configured a TLS certificate, a shared secret to secure communications between the Hub and Proxy components and an authentication provider, we can deploy JuptyerHub to our Kubernetes cluster.

First, we need to configure Helm to add the JupyterHub repository:

```shell
helm repo add jupyterhub https://jupyterhub.github.io/helm-chart/
helm repo update
```

Next, we use Helm to deploy JupyterHub. This will also pull the single user environment Docker image for running analysis as part of the deployment, so that there isn't the delay of pulling this image when a user logs in.

```shell
helm upgrade --install jupyterhub jupyterhub/jupyterhub \
  --namespace default \
  --version=0.8.2 \
  --values jupyterhub-values.yml
```

After a few minutes, you should now be able to visit https://kiln-jupyterhub.my-subdomain.mydomain.tld and login using the authentication mechanism you configured earlier.

## Generating Test Data

In order to generate test data for analysis, we need to run Kiln over every commit on the master branch of several git repositories. To automate this process, the repo-analyser.py python script is provided. This script will checkout each commit to master in reverse chronological order, write the required kiln.toml file to the repo, then use the Kiln CLI to run bundler-audit over the project and send the results to your Kiln stack.

Before you can run the script, you will need to install it's dependencies by running: `pipenv sync`. Then you will need to run the script three times, once each for RailsGoat, Mastodon and GitLab.

```shell
git clone https://github.com/OWASP/railsgoat.git
pipenv run python3 repo-analyser.py ./railsgoat railsgoat https://kiln-data-collector.my-subdomain.mydomain.tld
```

```shell
git clone https://github.com/tootsuite/mastodon.git
pipenv run python3 repo-analyser.py ./mastodon mastodon https://kiln-data-collector.my-subdomain.mydomain.tld
```

```shell
git clone https://gitlab.com/gitlab-org/gitlab.git
pipenv run python3 repo-analyser.py ./gitlab gitlab https://kiln-data-collector.my-subdomain.mydomain.tld
```

These could take quite a while to run and you will see the occasional error message about there not being a Gemfile.lock to analyse. This is caused by the Gemfile.lock either not existing in that commit, or sometimes being broken by a bad merge. Additionally, you may experience occasional network problems, but these will not significantly affect the data generated.

## Performing Data Analysis
