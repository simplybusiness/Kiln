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

Make a note of the output from the final command that creates the API access keys for the `kops` user, particularly `SecretAccessKey` and `AccessKeyId` and be sure to store these securely. We will need them later, and exposing them would grant an attacker access to your AWS account.
