#!/usr/bin/env python3
import click
import os
from pygit2 import discover_repository, GIT_STATUS_CURRENT, GIT_STATUS_IGNORED, Repository
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
    working_copy_clean = check_for_expected_working_copy_changes()
    if working_copy_clean == False:
        raise click.UsageError("Working copy contains uncomitted changes except for CHANGELOG.md")
    pass

def check_for_expected_working_copy_changes():
    current_working_directory = os.getcwd()
    repository_path = discover_repository(current_working_directory)
    repo = Repository(repository_path)
    status = repo.status()
    for filepath, flags in status.items():
        if flags != GIT_STATUS_CURRENT and flags != GIT_STATUS_IGNORED and filepath != "CHANGELOG.md":
            return False
    return True

if __name__ == "__main__":
    main()
