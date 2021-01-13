#!/usr/bin/env python3
import click
import re
import semver

def validate_version_number(ctx, param, value):
    try:
        return semver.VersionInfo.parse(value)
    except TypeError:
        raise click.BadParameter("Version number not semver compatible")
    except ValueError:
        raise click.BadParameter("Version number not semver compatible")

@click.command()
@click.option('--version', required=True, prompt="Version number to release", callback=validate_version_number)
@click.password_option('--signing-key-passphrase', prompt="Release signing key passphrase") 
@click.confirmation_option()
def main(version, signing_key_passphrase):
    pass

if __name__ == "__main__":
    main()
