#!/usr/bin/env python3
import click
import re

def validate_version_number(ctx, param, value):
    matches = re.match(r"v?(\d+\.\d+\.\d+)", value)
    if matches == None:
        raise click.BadParameter("Version number must be in format [v]1.2.3")
    return matches.group(1)

@click.command()
@click.option('--version', required=True, prompt="Version number to release", callback=validate_version_number)
@click.password_option('--signing-key-passphrase', prompt="Release signing key passphrase") 
@click.confirmation_option()
def main(version, signing_key_passphrase):
    pass

if __name__ == "__main__":
    main()
