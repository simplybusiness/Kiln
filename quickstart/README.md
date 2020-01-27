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
