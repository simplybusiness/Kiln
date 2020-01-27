# Kiln Quickstart

This document aims to get you up and running with a Kiln stack fairly quickly and assume you will be deploying it to an AWS environment using a Kubernetes cluster which this document will guide you through setting up.

**Note: This configuration has not been production tested and does not make any guarantees about the safety or availability of the data it will host. Please think very carefully about the configuration choices made before deploying Kiln to production.**

## Objectives
* A Kubernetes cluster provisioned in AWS using the Kops tool, capable of hosting components in a HA configuration
* Kubernetes nodes sized appropriately for the components they'll be hosting, plus a group of larger nodes for data analysis
* A Kiln stack deployed to the Kubernetes cluster, with the following components: Data-collector, Report-parser, Kafka, Zookeeper & Slack-connector
* A Jupyterhub stack configured for data analysis
* Data generated from 3 open source ruby projects to practise analysis on (based on a talk given as BSides Leeds 2020 by Dan Murphy)

## Prerequisites
* AWS CLI tools installed - Instructions can be found here: [https://docs.aws.amazon.com/cli/latest/userguide/install-cliv1.html](https://docs.aws.amazon.com/cli/latest/userguide/install-cliv1.html)
* Kubectl installed - Instructions can be found here: [https://kubernetes.io/docs/tasks/tools/install-kubectl/](https://kubernetes.io/docs/tasks/tools/install-kubectl/)
* Kops installed - Instructions can be found here: [https://github.com/kubernetes/kops/blob/master/docs/install.md](https://github.com/kubernetes/kops/blob/master/docs/install.md)
* Helm installed - Instructions can be found here: [https://helm.sh/docs/intro/install/](https://helm.sh/docs/intro/install/)
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

Now we have an IAM user created with the necessary permissions to use `kops`, we need to setup the DNS domain that will be used for our Kubernetes cluster, which we assume is already hosted in Route53. The simplest approach is to add records to the root of a domain, where all subdomains related to the cluster will be in the form `something.clustername.mydomain.tld`. If this is appropriate for your environment, then you don't need to do anything else here.

If you want to create all cluster subdomains under a specific subdomain under your domain name (taking the form `something.clustername.subdomain.mydomain.tld`), then you will need to create a new hosted zone in Route53 and setup an NS record for this subdomain in the parent domain.

Note: these instructions assume you have [jq](https://stedolan.github.io/jq/) installed.

* Create the subdomain hosted zone in Route53, make a note of the output of this command. It is the Nameservers for the subdomain, which you will need later.

``` shell
ID=$(uuidgen) && aws route53 create-hosted-zone --name subdomain.example.com --caller-reference $ID | jq .DelegationSet.NameServers
```

* Find your parent hosted zone ID

``` shell
aws route53 list-hosted-zones | jq '.HostedZones[] | select(.Name=="mydomain.tld.") | .Id'
```

* Create a configuration file with your **subdomain** nameservers, replacing the domains containing `awsdns` with the values you made a note of earlier

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

* Ensure your NS records have been configured correctly by running the following command. If the correct nameservers are not returned, do not proceed. Correct DNS configuration is critical to the following steps. This step is not required if you are using a bare domain for your cluster.

``` shell
dig ns mysubdomain.mydomain.tls
```
