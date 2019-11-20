#!/bin/bash
#Step 0
# Delete existing tls folder
rm -rf tls
mkdir tls
cp ssl.cnf tls/
cp client-ssl.properties tls/
pushd tls
#Step 1
#Generate server keystore
keytool -keystore kafka.server.keystore.jks -alias localhost -validity 365 -genkey -keyalg rsa -storepass password -keypass password -dname "C=GB, ST=London, O=Kiln, OU=Integration Testing, CN=Kafka"
#Step 2
#Create CA
openssl req -new -x509 -keyout ca-key -out ca-cert -days 365 -nodes -subj "/C=GB/ST=London/O=Kiln/OU=Integration Test/CN=Kafka CA"
#Add generated CA to the trust store
keytool -keystore kafka.server.truststore.jks -alias CARoot -import -file ca-cert -storepass password
#Step 3
#Sign the key store
keytool -keystore kafka.server.keystore.jks -alias localhost -certreq -file cert-file -storepass password -sigalg SHA256withRSA
openssl x509 -req -CA ca-cert -CAkey ca-key -in cert-file -out cert-signed -days 365 -CAcreateserial -passin pass:password -extfile ssl.cnf -extensions req_ext -sha256
keytool -keystore kafka.server.keystore.jks -alias CARoot -import -file ca-cert -storepass password
keytool -keystore kafka.server.keystore.jks -alias localhost -import -file cert-signed -storepass password 
popd
