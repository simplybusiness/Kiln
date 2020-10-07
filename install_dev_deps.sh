#!/bin/sh
echo "[+] Kiln Installation Script..."
echo -e "Do you plan to use the slack connector? [Y/n] \c"
read slack
if [ -z $slack ] || [ $slack = "y" ] || [ $slack = "Y" ]; then
    echo -e "Please provide OAUTH2 token: \c"
    read OAUTH2
    echo -e "Please provide the channel ID for default messaging: \c"
    read CHID
    echo "OAUTH2_TOKEN=$OAUTH2" > .env
    echo "SLACK_CHANNEL=$CHID" >> .env
    echo "[+] Slack information written to .env file"
else
    echo "[+] You can change this later by adding the auth and slack channel information to a .env file in the kiln root."
fi
ARCH=`uname`
# MacOS install
if [ $ARCH = "Darwin" ]; then
    echo "[+] MacOS detected... Installing any missing dependencies."
    
    if [ -z $(which docker) ]; then
        echo "[-] Docker not found.  Installing docker cask with homebrew."
        brew cask install docker
    fi    
    if [ -z $(which rustc) ]; then
        echo "[-] Rustc not found. Installing rust with homebrew."
        brew install rust
    fi
    if [ -z $(which java) ]; then
        echo "[-] Java not found.  Installing openjdk with homebrew."
        brew install openjdk
    fi
    if [ -z $(which docker-compose) ]; then
        echo "[-] docker-comopose not found. Installing docker-compose with homebrew."
        brew install docker-compose
    fi
    if [ -z $(which openssl) ]; then
        echo "[-] openssl not found. Installing openssl with homebrew."
        brew install openssl
    fi
#linux install
elif [ $ARCH = "Linux" ]; then
    echo "[+] Linux system detected... Installing any missing dependencies."
    PACK=`which apt-get`
    if  [ -z $PACK ]; then
        PACK=`which yum`
        if [ -z $PACK ]; then
            echo "[-] Unable to find a package manager.  Install failed"
            exit 1
        fi
        PACKAGES="-y install rustc openjdk-8-jdk openssl docker.io docker-compose" 
    else
        $PACK update
        PACKAGES="install -y docker.io rustc openjdk-8-jdk openssl docker-compose"
    fi 
    
    $PACK $PACKAGES
    systemctl start docker
fi
echo "[+] Dependencies have been installed.  Proceeding to next step."
echo "[+] Installing cargo make..."
cargo install cargo-make
echo "[+] Generating certificates..."
./gen_certs.sh
echo "[+] Making tools."
cargo make tools
echo "[+] Making server components...  This may take some time. You should get some coffee or play solitaire for a bit."
cd data-collector
cargo make build-data-collector-git-docker
cd ../report-parser
cargo make build-report-parser-docker
cd ../slack-connector
cargo make build-slack-connector-docker
cd ../data-forwarder
cargo make build-data-forwarder-musl
cd ..
echo "[+] Making kiln-cli binary"
cargo make cli
echo "[+] Installation complete."
echo "[+] run ./start-local.sh to start the docker containers."
