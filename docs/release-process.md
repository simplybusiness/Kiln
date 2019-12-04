# Setting up to release Kiln

In order to deploy Kiln, we make use of signed git tags to provide assurance of authenticity that a release was made by a Kiln maintainer. The release CI will ensure that a release tag was signed using one of a set of a GPG keys that have been added to repo.

To generate a GPG Keypair to sign releases:

[![GPG Key Generation](https://asciinema.org/a/zuHiqVnXFXRJgenoRuO4KRNL1.svg)](https://asciinema.org/a/zuHiqVnXFXRJgenoRuO4KRNL1)

``` Shell
gpg --full-gen-key --expert                                                                                                                                                                                                              
gpggpg (GnuPG) 2.2.18; Copyright (C) 2019 Free Software Foundation, Inc.
This is free software: you are free to change and redistribute it.
There is NO WARRANTY, to the extent permitted by law.

Please select what kind of key you want:                                                                                                                                                                                                      
   (1) RSA and RSA (default)
   (2) DSA and Elgamal
   (3) DSA (sign only)
   (4) RSA (sign only)
   (7) DSA (set your own capabilities)
   (8) RSA (set your own capabilities)
   (9) ECC and ECC
  (10) ECC (sign only)
  (11) ECC (set your own capabilities)
  (13) Existing key
  (14) Existing key from card
Your selection? 9
Please select which elliptic curve you want:
   (1) Curve 25519
   (3) NIST P-256
   (4) NIST P-384
   (5) NIST P-521
   (6) Brainpool P-256
   (7) Brainpool P-384
   (8) Brainpool P-512
   (9) secp256k1
Your selection? 1
Please specify how long the key should be valid.
         0 = key does not expire
      <n>  = key expires in n days
      <n>w = key expires in n weeks
      <n>m = key expires in n months
      <n>y = key expires in n years
Key is valid for? (0)
Key does not expire at all
Is this correct? (y/N) y

GnuPG needs to construct a user ID to identify your key.
 
Real name: Daniel Murphy
Email address: danhatesnumbers@gmail.com
Comment:

You selected this USER-ID:
    "Daniel Murphy <danhatesnumbers@gmail.com>"
 
Change (N)ame, (C)omment, (E)mail or (O)kay/(Q)uit?

gpg --export -a danhatesnumbers@gmail.com

-----BEGIN PGP PUBLIC KEY BLOCK-----
 
mDMEXeb4ABYJKwYBBAHaRw8BAQdAjuSU29UAGBoharr17YcRfl6cc4DR6pzoKcNN
z276jti0KURhbmllbCBNdXJwaHkgPGRhbmhhdGVzbnVtYmVyc0BnbWFpbC5jb20+
iJAEExYIADgWIQQkyH9eWbyZhIEs76wqK3I254DMBwUCXeb4AAIbAwULCQgHAgYV
CgkICwIEFgIDAQIeAQIXgAAKCRAqK3I254DMB7xHAP9htMrmomFxH+ymCeUMimNi
y6YhbrqmX9ugWtydg9wyMAD/UYrleAH+hcAZAHB9/bZBRwY+m4Q66ppzJbhPBVHN
Gg+4OARd5vgAEgorBgEEAZdVAQUBAQdAU2JiW17IJyTECfuCAZ7lEuSU05iEpIUy
Clk8MZkCSE4DAQgHiHgEGBYIACAWIQQkyH9eWbyZhIEs76wqK3I254DMBwUCXeb4
AAIbDAAKCRAqK3I254DMBwwCAQDEtG/TWUjzrX+WRbMez2PLxGfY5p+gqRBpSCxc
gjuwpQD/SbBEwd5YrBkWTDa8ce8Xdz+jokKhl8tXg8R4jWTkug6YMwRd6B9tFgkr
BgEEAdpHDwEBB0AgSZJDFzOZ02o7FlYitbUSt1Bns0iVD4qCAhNN584gtLQpRGFu
aWVsIE11cnBoeSA8ZGFuaGF0ZXNudW1iZXJzQGdtYWlsLmNvbT6IkAQTFggAOBYh
BOS8O9KF4GaJQpjl2zd6Do+MWsWuBQJd6B9tAhsDBQsJCAcCBhUKCQgLAgQWAgMB
Ah4BAheAAAoJEDd6Do+MWsWuT6sBAL39/pT+6CsiUSSo4QfbA/3isYmmwBljCaqu
1HElI1dOAP9sIWEW2PViCqanBd13bE572EA4vTZNf2UyAiQXmmEnDbg4BF3oH20S
CisGAQQBl1UBBQEBB0A4pRZyeZ/+narvFJrRkHIkh6KIMtVfURiYYMFpYcsHQQMB
CAeIeAQYFggAIBYhBOS8O9KF4GaJQpjl2zd6Do+MWsWuBQJd6B9tAhsMAAoJEDd6
Do+MWsWu1jkA/RBKM4qzbzrs7qRmO3qC3FGE+dEokkRzJh5gEHNLMDJFAP4hxLzG
pW+dkjGd7r5rGCFvytGBgFSH/aQ1P54Hy+HoBQ==
=MPrc
-----END PGP PUBLIC KEY BLOCK-----
```

If you own a Yubikey, you can move your private keys into your Yubikey and keep a backup offline to keep your signing keys as secure as possible. Yubico have great instructions on how to backup and move your keys to your Yubikey [https://developers.yubico.com/PGP/Importing_keys.html](https://developers.yubico.com/PGP/Importing_keys.html).

Once you have your signing public key exported in ascii-armoured format, raise a PR with the key stored in `meta/release-keys/<yourname>.pub`.

# Releasing Kiln

* Open a release branch from master
* Update version number in all crates Cargo.toml files with next semver version number based on changes
* Swap dependencies on Kiln_lib from git-master to git-tag
* Update CHANGELOG.md
* Tag commit with SemVer version number and sign using your GPG key
* Push commits and tag
* Revert kiln_lib dependencies from git-tag back to git-master
* Merge to master

Once you have completed these steps, Github Actions will trigger the release workflow, which will do the following:
* Checkout the repo to the latest release tag
* Ensure GPG installed
* Import release public keys from `meta/release-keys`
* Verify signature on the release tag to ensure it was created by a trusted releaser
* Zip contents of repo and upload to Github Releases
* Build CLI app on Windows, Linux and MacOS and upload to Github Releases
* Build all docker images with latest, datestamp and SemVer tags (including the version of tools where appropriate) and push to Docker Hub
