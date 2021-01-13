#!/usr/bin/env python3
import click
import os
from pygit2 import discover_repository, GIT_CHECKOUT_ALLOW_CONFLICTS, GIT_CHECKOUT_SAFE, GIT_STATUS_CURRENT, GIT_STATUS_IGNORED, Repository
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
    kiln_repo = find_repo()
    head_of_main = kiln_repo.revparse_single('main')
    working_copy_clean = check_for_expected_working_copy_changes(kiln_repo)
    if working_copy_clean == False:
        raise click.UsageError("Working copy contains uncomitted changes except for CHANGELOG.md")
    release_branch = kiln_repo.branches.local.create(f"release/{version}", head_of_main)
    kiln_repo.checkout(release_branch, strategy=GIT_CHECKOUT_SAFE|GIT_CHECKOUT_ALLOW_CONFLICTS)

def find_repo():
    current_working_directory = os.getcwd()
    repository_path = discover_repository(current_working_directory)
    repo = Repository(repository_path)
    return repo

def check_for_expected_working_copy_changes(kiln_repo):
    status = kiln_repo.status()
    for filepath, flags in status.items():
        if flags != GIT_STATUS_CURRENT and flags != GIT_STATUS_IGNORED and filepath != "CHANGELOG.md":
            return False
    return True

if __name__ == "__main__":
    main()
