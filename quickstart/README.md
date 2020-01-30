# Kiln Quickstart

This document aims to get you up and running with a Kiln stack fairly quickly and assume you will be deploying it to an AWS environment using a Kubernetes cluster which this document will guide you through setting up. Although these instructions cover deploying Zookeeper and Kafka, it should be noted that administering these components in production is a complex, time consuming job. Unless you already have Zookeeper and Kafka administration experience, it is strongly suggested that you use a managed service such as [AWS Managed Streaming for Kafka](https://aws.amazon.com/msk/) or the [Confluent Platform](https://www.confluent.io/confluent-cloud).

**Note: This configuration has not been production tested and does not make any guarantees about the safety or availability of the data it will host. Please think very carefully about the configuration choices made before deploying Kiln to production.**

## Objectives
* A Kubernetes cluster provisioned in AWS using the Kops tool, capable of hosting components in a HA configuration
* Kubernetes nodes sized appropriately for the components they'll be hosting, plus a group of larger nodes for data analysis
* A Kiln stack deployed to the Kubernetes cluster, with the following components: Data-collector, Report-parser, Kafka, Zookeeper & Slack-connector

## Prerequisites
* AWS CLI tools installed - Instructions can be found here: [https://docs.aws.amazon.com/cli/latest/userguide/install-cliv1.html](https://docs.aws.amazon.com/cli/latest/userguide/install-cliv1.html)
* Kubectl installed - Instructions can be found here: [https://kubernetes.io/docs/tasks/tools/install-kubectl/](https://kubernetes.io/docs/tasks/tools/install-kubectl/)
* Kops installed - Instructions can be found here: [https://github.com/kubernetes/kops/blob/master/docs/install.md](https://github.com/kubernetes/kops/blob/master/docs/install.md)
* Helm installed - Instructions can be found here: [https://helm.sh/docs/intro/install/](https://helm.sh/docs/intro/install/)
* LibreSSL or OpenSSL (for generating a Certificate Authority and TLS certificates)
* Java Developer Kit (for generating Java Keystore files for Kafka using KeyTool) - Instructions can be found here: [https://adoptopenjdk.net/installation.html?variant=openjdk11&jvmVariant=hotspot](https://adoptopenjdk.net/installation.html?variant=openjdk11&jvmVariant=hotspot)
* An AWS account and an IAM user with permissions to create a new IAM user
* A domain name hosted in AWS Route53

## AWS Configuration

### IAM user

We need to create an IAM user that we can use to run the Kops tool to provision our Kubernetes cluster. If you have an existing AWS environment, you could also achieve this by attaching the appropriate managed policies (documented below) to an existing User, Group of Users or Role and use that instead of creating a new IAM user.

We're going to create a group named `kops`, attach permissions for managing EC2, Route53, S3, IAM and VPCs to the group, create a new user named `kops` and add them to the group we just created and create a set of API keys for this user.

``` shell
aws iam create-group --group-name kops

aws iam attach-group-policy --policy-arn arn:aws:iam::aws:policy/AmazonEC2FullAccess --group-name kops
aws iam attach-group-policy --policy-arn arn:aws:iam::aws:policy/AmazonRoute53FullAccess --group-name kops
aws iam attach-group-policy --policy-arn arn:aws:iam::aws:policy/AmazonS3FullAccess --group-name kops
aws iam attach-group-policy --policy-arn arn:aws:iam::aws:policy/IAMFullAccess --group-name kops
aws iam attach-group-policy --policy-arn arn:aws:iam::aws:policy/AmazonVPCFullAccess --group-name kops

aws iam create-user --user-name kops

aws iam add-user-to-group --user-name kops --group-name kops

aws iam create-access-key --user-name kops
```

Make a note of the output from the final command that creates the API access keys for the `kops` user, particularly `SecretAccessKey` and `AccessKeyId` and be sure to store these securely, exposing them would grant an attacker access to your AWS account.

Next, we'll setup the `kops` user in the AWS CLI credentials and configuration files, so that it can easily be used in subsequent steps.

In `~/.aws/config`, add the following block of text, replacing the region ID with the region you wish to build your cluster in:
```
[profile kops]
region = eu-west-2
```

In `~/.aws/credentials`, add the following block of text, replacing the Access Key ID and Secret Access Key with the values you noted earlier.
```
[kops]
aws_access_key_id     = AKIAIOSFODNN7EXAMPLE
aws_secret_access_key = wJalrXUtnFEMI/K7MDENG/bPxRfiCYzEXAMPLEKEY
```

Now, the `kops` user can be used for our subsequent steps by exporting the following environment variable: `export AWS_PROFILE=kops`.

### Configuring DNS

Now we have an IAM user created with the necessary permissions to use `kops`, we need to setup the DNS domain that will be used for our Kubernetes cluster, which we assume is already hosted in Route53. In order to keep cluster resources relatively self-contained, we will create a subdomain to contain all of our cluster DNS records, taking the form `something.clustername.subdomain.mydomain.tld`). To do this, you will need to create a new hosted zone in Route53 and setup an NS record for this subdomain in the parent domain.

Note: these instructions assume you have [jq](https://stedolan.github.io/jq/) installed.

* Create the subdomain hosted zone in Route53, make a note of the output of this command. It is the Nameservers for the subdomain, which you will need later.

``` shell
ID=$(uuidgen) && aws route53 create-hosted-zone --name subdomain.example.com --caller-reference $ID | jq .DelegationSet.NameServers
```

* Find your parent hosted zone ID

``` shell
aws route53 list-hosted-zones | jq '.HostedZones[] | select(.Name=="mydomain.tld.") | .Id'
```

* Create a configuration file with your **subdomain** nameservers, replacing the domains in the "ResourceRecords" list with the values you made a note of earlier

``` json
{
  "Comment": "Create a subdomain NS record in the parent domain",
  "Changes": [
    {
      "Action": "CREATE",
      "ResourceRecordSet": {
        "Name": "subdomain.mydomain.tld",
        "Type": "NS",
        "TTL": 300,
        "ResourceRecords": [
          {
            "Value": "ns-1.awsdns-1.co.uk"
          },
          {
            "Value": "ns-2.awsdns-2.org"
          },
          {
            "Value": "ns-3.awsdns-3.com"
          },
          {
            "Value": "ns-4.awsdns-4.net"
          }
        ]
      }
    }
  ]
}
```

* Create an NS record in the parent hosted zone, delegating name resolution for the subdomain to the correct name servers

``` shell
aws route53 change-resource-record-sets --hosted-zone-id <parent-zone-id> --change-batch file://<path to subdomain config file from previous step>.json
```

* Ensure your NS records have been configured correctly by running the following command, but bear in mind that DNS record propogation means this could take some time to return the correct answer. If the correct nameservers are not returned, do not proceed. Correct DNS configuration is critical to the following steps.

``` shell
dig ns mysubdomain.mydomain.tld
```

## Kops Cluster state storage

We're going to setup an AWS S3 bucket for `kops` to store the state of the cluster it provisions, so that it can keep track of resources it has created. We're also going to ensure that versioning is enabled as well as server-side encryption. It's important to remember that S3 bucket names must be globally unique, so bear this in mind when naming your cluster state storage bucket.

``` shell
aws s3api create-bucket --bucket my-cluster-state-storage-bucket --region eu-west-2 --create-bucket-configuration LocationConstraint=eu-west-2
aws s3api put-bucket-versioning --bucket my-cluster-state-storage-bucket --versioning-configuration Status=Enabled
aws s3api put-bucket-encryption --bucket my-cluster-state-storage-bucket --server-side-encryption-configuration '{"Rules":[{"ApplyServerSideEncryptionByDefault":{"SSEAlgorithm":"AES256"}}]}'
```

## Bootstrapping the Kubernetes cluster

Now we're ready to bootstrap the Kubernetes cluster that we'll be deploying Kiln into. The commands below will bootstrap a cluster with a Highly Available control plane, 3 worker nodes, using t3.medium EC2 instances in the eu-west-2 region. Additionally, they will setup CoreDNS for providing DNS for cluster nodes and attaching the required IAM policy for ExternalDNS to configure external DNS records in a lter step.

``` shell
export NAME=my-cluster-name
export KOPS_STATE_STORE=s3://my-cluster-state-storage-bucket
export AWS_PROFILE=kops
kops create cluster \
    --node-count 3 \
    --zones eu-west-2a,eu-west-2b,eu-west-2c \
    --master-zones eu-west-2a,eu-west-2b,eu-west-2c \
    --node-size t3a.medium \
    --master-size t3a.medium \
    --topology public \
    --networking calico \
    ${NAME}
kops edit cluster ${NAME}
```

This will create the cluster configurations, but won't apply them just yet. The last command in the above block will open your configured terminal editor to make some changes before we stand up the cluster. Your editor should contain a YAML document, locate the `spec` key at the top level of the document and insert the following snippet (replacing the Hosted Zone ID in the IAM policy with the Hosted Zone ID you will be hosting cluster DNS records under and being careful to maintain proper indentation):

``` YAML
spec:
  kubeDNS:
    provider: CoreDNS

  additionalPolicies:
    node: |
      [
       {
          "Effect": "Allow",
          "Action": [
            "route53:ChangeResourceRecordSets"
          ],
          "Resource": [
            "arn:aws:route53:::hostedzone/MYHOSTEDZONEID"
          ]
        },
        {
          "Effect": "Allow",
          "Action": [
            "route53:ListHostedZones",
            "route53:ListResourceRecordSets"
          ],
          "Resource": [
            "*"
          ]
        }
      ]
```

Save the updated cluster spec and exit your editor. Now it's time to bring your cluster up.

``` shell
kops update cluster ${NAME} #Use this to preview the changes you're about to make
kops update cluster ${NAME} --yes #Run this once you're happy for the changes to be applied
kops validate cluster ${NAME} --wait 30s #This will check the state of your cluster every 30 seconds and exit once the cluster is fully operational. Cluster creation involves creating DNS records, so this might take a while.
```

## Deploying Kiln supporting services
### ExternalDNS
ExternalDNS is a cluster add-on provided by the Kubernetes ExternalDNS Special Interest Group (SIG) and instructions for customising how this is deployed can be found at [https://github.com/kubernetes-sigs/external-dns/blob/master/docs/tutorials/aws.md](https://github.com/kubernetes-sigs/external-dns/blob/master/docs/tutorials/aws.md). We're going to be using a Kubernetes manifest adapted from their example manifest for a cluster using Role-based Access Control (RBAC), you can find the original manifest at [https://github.com/kubernetes-sigs/external-dns/blob/master/docs/tutorials/aws.md#manifest-for-clusters-with-rbac-enabled](https://github.com/kubernetes-sigs/external-dns/blob/master/docs/tutorials/aws.md#manifest-for-clusters-with-rbac-enabled).

The adapted manifest can be found at [external-dns.yaml](./external-dns.yaml). Ensure that you replace the Zone Filter domain list so that it matches the domain you will be hosting your Kubernetes cluster DNS entries under, then run the following to apply the changes: `kubectl apply -f external-dns.yaml`.

### Zookeeper
To deploy Zookeeper, which Kafka requires for conducting leadership elections, we're going to use the Helm package manager for Kubernetes. [Bitnami](https://bitnami.com/) provide a number of production ready packaged applications for a number of platforms, including Helm charts for Kubernetes. We'll configure Helm to include the Bitnami package repository, then deploy a Zookeeper stack using the Bitnami Helm chart.

``` shell
helm repo add bitnami https://charts.bitnami.com/bitnami
helm install zk -f zookeeper-values.yaml bitnami/zookeeper
```

### Kafka
Deploying Kafka involves 4 steps: Generating the TLS certificates used to secure connections, creating a kubernetes secrets for the TLS certificates, deploying the Kafka chart and finally creating the topics required for Kiln.

This document assumes you do not have an existing PKI you want to use to generate the Kafka TLS certificates, so it will guide you through creating a small PKI. PKI configurations can be extremely varied, so thier usage is out of scope for this document. You should make sure the private key for the CA Certificate you will be generating is stored securely. We will be using a customised version of the Bitnami Kafka Helm Chart. The reason for the customisation is because Kiln currently does not support authenticating to a Kafka cluster (work to add support for this is being tracked in https://github.com/simplybusiness/Kiln/issues/169), and the upstream Helm Chart does not allow configuring TLS without also requiring authentication.

* Generating Kakfa Certificate Authority and TLS certificate. Answer "yes" when prompted both times if you trust this certificate. This shell script creates the CA, generates a certificate request for the Kafka server certificate and builds the Java Keystore for the signed certificate and a separate Java Keystore for the CA certificate. Resulting files will be in the `tls` directory.
``` shell
./gen_certs.sh 
```

* Create the required Kubernetes secrets
``` shell
cd tls
kubectl create secret generic kafka-certs --from-file=./kafka.truststore.jks --from-file=./kafka.keystore.jks
kubectl create secret generic kafka-ca --from-file=./ca-cert
```

* Deploying Kafka
``` shell
cd kafka
helm install kafka ./ -f kafka-values.yaml
kubectl get pods -w -l app.kubernetes.io/name=kafka # Wait for pods to be ready
```

* Create the Kafka topics for Kiln. The last command in the following block will print the list of Kafka topics that exist in this cluster, it should now contain "ToolReports" and "DependencyEvents"
``` shell
export POD_NAME=$(kubectl get pods --namespace default -l "app.kubernetes.io/name=kafka,app.kubernetes.io/instance=kafka,app.kubernetes.io/component=kafka" -o jsonpath="{.items[0].metadata.name}")
kubectl --namespace default exec -it $POD_NAME -- kafka-topics.sh --create --zookeeper zk-zookeeper-headless:2181 --replication-factor 3 --partitions 3 --topic ToolReports
kubectl --namespace default exec -it $POD_NAME -- kafka-topics.sh --create --zookeeper zk-zookeeper-headless:2181 --replication-factor 3 --partitions 3 --topic DependencyEvents
kubectl --namespace default exec -it $POD_NAME -- kafka-topics.sh --list --zookeeper zk-zookeeper-headless:2181
```

## Deploying Kiln
### Mandatory components
The mandatory for components for a Kiln deployment are the Data-collector and Report-parser. Before you can deploy these components, you will need to request a TLS certificate for the Data-collector. AWS ACM provides free, auto-renewing TLS certificates that are publically trusted, so for this quickstart, follow the documentation that AWS provide, which can be found at: [https://docs.aws.amazon.com/acm/latest/userguide/gs-acm-request-public.html](https://docs.aws.amazon.com/acm/latest/userguide/gs-acm-request-public.html). When prompted for a domain, use "kiln-data-collector.mydomain.tld", replacing the top level domain with the domain you configured earlier.

Once your ACM certificate has been issued, take a note of it's ARN and then replace the example ARN in data-collector.yaml with the ARN for your new certificate. The key for this value is `Metadata->Annotations->service.beta.kubernetes.io/aws-load-balancer-ssl-cert` in the YAML block for the Service. In the same set of annotations, you will also need to fill in the correct value for the external DNS record you want to be created for the Data-collector.

Once you've acquired and configured your ACM certificate and filled in the correct DNS name, the mandatory components can be deployed by running the following commands:

``` shell
kubectl apply -f data-collector.yaml
kubectl apply -f report-parser.yaml
```

### Slack connector (optional)
If you want Kiln to send notifications to a Slack channel when issues are discovered, you will need to register a Slack developer application to obtain an OAuth2 token to authenticate to Slack, create a Kubernetes secret to securely deliver this token to the Slack-connector component, then deploy the Slack-connector component itself.

* Create a Slack Developer App by following the first 3 sections of this Slack Developer documentation: [https://api.slack.com/authentication/basics](https://api.slack.com/authentication/basics). These instructions should get you as far as finding the OAuth2 token in the App Management page. A note on token scopes, when you are requesting scopes for your Slack OAuth2 token, be sure to select the `channels:read` and `chat:write` scopes to the Bot Token (not the User Token!), to limit access to just what Kiln requires.

* Once you have your OAuth2 token, write it to a `.env` file in the following format:
```
OAUTH2_TOKEN=mytokenhere
```

* Now you can create a Kubernetes secret to securely storeand deliver this token to the Slack-connector component as an environment variable
``` shell
kubectl create secret generic slack-oauth-token --from-env-file=path/to/.env
```

* Next you need to find the Channel ID of the channel you want to send notifications to. Currently, Kiln only supports sending messages to a single channel, but work to enhance this functionality and add support for conditional routing is being tracked in [https://github.com/simplybusiness/Kiln/issues/154](https://github.com/simplybusiness/Kiln/issues/154). The easiest way to do this is open the channel in the Slack web app and the Channel ID will be the last segment of the URL path, starting with a 'C'. Once you have the Channel ID, replace the Channel ID environment variable in `slack-connector.yaml`.

* Finally, you can deploy the Slack-connector
``` shell
kubectl apply -f slack-connector.yaml
```

## Cleanup

Once you're finished experimenting with Kiln, you should clean up the resources you created in this quickstart to ensure you aren't charged for resources you aren't using.

AUTHOR NOTE:
* Cluster
* Route53 entries
* ACM certificates
* EBS volumes
* S3 state bucket

``` shell

```
