#!/bin/sh
trap '{ echo "Kiln shutdown initiated... Stopping containers..."; for j in $(jobs -lp); do jobs -l; kill $j; done; sleep 5; exit 1; }' INT
docker-compose up zookeeper kafka & 
sleep 10
docker-compose up data-collector report-parser &
sleep 20
echo "================================================================================================="
echo "[+] Docker containers for zookeeper, kafka, data-collector and report-parser have been started..."
echo "[*] Do you want to start the slack-connector? [Y/n]"
while true; do
    read yn
    if [ $yn = "Y" ] || [ $yn = "y" ] || [ -z $yn ]; then
        docker-compose up slack-connector &
        sleep 5
        echo "[+] slack-connector has been started"
        break
    fi
    if [ $yn = "N" ] || [ $yn = "n" ]; then
        break
    fi
done
echo "[+] Kiln has been started.  Use kiln-cli to scan an application. Use Ctrl-C to shutdown Kiln"

while true; do
    sleep 1
done