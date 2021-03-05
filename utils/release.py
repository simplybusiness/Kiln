#!/usr/bin/env python3
import click
import docker
import dulwich.porcelain
from dulwich.repo import Repo
import hashlib
import gpg
import os
from pathlib import PurePath
import semver
import sh
import shutil
import sys
import tarfile
import toml
from toml import TomlPreserveInlineDictEncoder
import io
import lzma
import whatthepatch
from github import Github


def validate_version_number(ctx, param, value):
    try:
        return semver.VersionInfo.parse(value)
    except TypeError:
        raise click.BadParameter("Version number not semver compatible")
    except ValueError:
        raise click.BadParameter("Version number not semver compatible")


@click.command()
@click.option('--version', required=True, prompt="Version number to release", callback=validate_version_number)
@click.option('--github-personal-access-token', required=True, envvar='GITHUB_PERSONAL_ACCESS_TOKEN')
@click.confirmation_option()
def main(version, github_personal_access_token):
    no_verify = True

    kiln_repo = Repo.discover()
    working_copy_clean = check_for_expected_working_copy_changes(kiln_repo)
    if not working_copy_clean:
        raise click.ClickException("Working copy contains uncomitted changes except for CHANGELOG.md")
    dulwich.porcelain.branch_create(kiln_repo.path, f"release/{version}")
    release_branch_ref = f"refs/heads/release/{version}".encode()
    kiln_repo.refs.set_symbolic_ref(b'HEAD', release_branch_ref)

    kiln_repo.stage(['CHANGELOG.md'])
    changelog_commit = kiln_repo.do_commit(message=f"Docs: Update CHANGELOG.md for {version} release.".encode(), no_verify=no_verify)

    buf = io.BytesIO()
    dulwich.porcelain.diff_tree(kiln_repo, kiln_repo.get_object(changelog_commit.parents[0]).tree, changelog_commit.tree, buf)

    diffs = whatthepatch.parse(buf.getvalue().decode("utf-8"))
    changelog_lines = []
    for diff in diffs:
        for change in diff.changes:
            if change.old is None and change.new is not None and change.line != "":
                changelog_lines.append(change.line)

    set_cargo_toml_version(kiln_repo, "kiln_lib", version)
    sh.cargo.check("--manifest-path", os.path.join(kiln_repo.path, "kiln_lib", "Cargo.toml"), "--all-features", _err=sys.stderr)
    kiln_repo.stage(['kiln_lib/Cargo.toml', 'kiln_lib/Cargo.lock'])
    kiln_lib_version_commit = kiln_repo.do_commit(message=f"Kiln_lib: Update component version to {version}".encode(), no_verify=no_verify)
    origin = kiln_repo.get_config().get(('remote', 'origin'), 'url')
    dulwich.porcelain.push(kiln_repo, remote_location=origin, refspecs=release_branch_ref)

    for component in ["data-collector", "data-forwarder", "report-parser", "slack-connector"]:
        set_kiln_lib_dependency(kiln_repo, component, sha=kiln_lib_version_commit)
        sh.cargo.check("--manifest-path", os.path.join(kiln_repo.path, component, "Cargo.toml"), "--all-features", _err=sys.stderr)
        kiln_repo.stage([f'{component}/Cargo.toml', f'{component}/Cargo.lock'])
        kiln_repo.do_commit(message=f"{component.capitalize()}: Update kiln_lib dependency to {version}".encode(), no_verify=no_verify)
        set_cargo_toml_version(kiln_repo, component, version)
        sh.cargo.check("--manifest-path", os.path.join(kiln_repo.path, component, "Cargo.toml"), "--all-features", _err=sys.stderr)
        kiln_repo.stage([f'{component}/Cargo.toml', f'{component}/Cargo.lock'])
        kiln_repo.do_commit(message=f"{component.capitalize()}: Update component version to {version}".encode(), no_verify=no_verify)

    set_cargo_toml_version(kiln_repo, "cli", version)
    sh.cargo.check("--manifest-path", os.path.join(kiln_repo.path, 'cli', "Cargo.toml"), "--all-features", _err=sys.stderr)
    kiln_repo.stage(['cli/Cargo.toml', 'cli/Cargo.lock'])
    kiln_repo.do_commit(message=f"CLI: Update component version to {version}".encode(), no_verify=no_verify)

    signing_key_id = kiln_repo.get_config()[(b'user',)][b'signingkey'].decode('utf-8')
    dulwich.porcelain.tag_create(kiln_repo, f"v{version}".encode(), message=f"v{version}".encode(), annotated=True, sign=signing_key_id)
    dulwich.porcelain.push(kiln_repo, remote_location=origin, refspecs=[release_branch_ref])
    dulwich.porcelain.push(kiln_repo, remote_location=origin, refspecs=[f"refs/tags/v{version}".encode()])

    sh.cargo.make("build-data-forwarder-musl", _cwd=os.path.join(kiln_repo.path, "data-forwarder"), _err=sys.stderr)
    shutil.copy2(os.path.join(kiln_repo.path, "bin", "data-forwarder"), os.path.join(kiln_repo.path, "tool-images", "ruby", "bundler-audit"))
    docker_client = docker.from_env()

    image_tags = docker_image_tags(version)
    (bundler_audit_image, build_logs) = docker_client.images.build(
            path=os.path.join(kiln_repo.path, "tool-images", "ruby", "bundler-audit"),
            tag=f"kiln/bundler-audit:{image_tags[0]}",
            rm=True)
    for line in build_logs:
        try:
            print(line['stream'], end='')
        except KeyError:
            pass

    for tag in image_tags[1:]:
        bundler_audit_image.tag("kiln/bundler-audit", tag=tag)

    for component in ["data-collector", "report-parser", "slack-connector"]:
        sh.cargo.make("musl-build", _cwd=os.path.join(kiln_repo.path, component), _err=sys.stderr)
        (docker_image, build_logs) = docker_client.images.build(
                path=os.path.join(kiln_repo.path, component),
                tag=f"kiln/{component}:{image_tags[0]}",
                rm=True)
        for line in build_logs:
            try:
                print(line['stream'], end='')
            except KeyError:
                pass
        for tag in image_tags[1:]:
            docker_image.tag(f"kiln/{component}", tag=tag)

    sh.cargo.make("musl-build", _cwd=os.path.join(kiln_repo.path, "cli"), _err=sys.stderr)
    base_path = os.path.join(kiln_repo.path, "cli", "target", "x86_64-unknown-linux-musl", "release")
    src_path = os.path.join(base_path, "kiln-cli")
    base_name = f"kiln-cli-{version}.x86_64"
    dst_path = os.path.join(base_path, base_name)
    tarball_name = f"{base_name}.tar.xz"
    tarball_path = os.path.join(base_path, tarball_name)
    hashfile_name = f"{tarball_name}.sha256"
    hashfile_path = os.path.join(base_path, hashfile_name)
    sig_name = f"{hashfile_name}.sig"
    sig_path = os.path.join(base_path, sig_name)

    os.rename(src_path, dst_path)
    with tarfile.open(name=tarball_path, mode='w:xz') as tarball:
        tarball.add(dst_path, arcname=base_name)

    sha256sum = hashlib.sha256()
    b = bytearray(128*1024)
    mv = memoryview(b)
    with open(tarball_path, 'rb', buffering=0) as f:
        for n in iter(lambda: f.readinto(mv), 0):
            sha256sum.update(mv[:n])
    tarball_hash = sha256sum.hexdigest()
    with open(hashfile_path, 'w') as f:
        f.write(f"{tarball_hash} {tarball_name}")

    with gpg.Context() as default_ctx:
        signing_key = default_ctx.get_key(signing_key_id)
        with gpg.Context(signers=[signing_key], armor=True) as ctx:
            with open(hashfile_path, 'rb') as hashfile:
                with open(sig_path, 'wb') as sigfile:
                    hashdata = hashfile.read()
                    sig, metadata = ctx.sign(hashdata, mode=gpg.constants.sig.mode.DETACH)
                    sigfile.write(sig)

    source_tarball_name = f"Kiln-{version}.tar.xz"
    source_tarball_path = os.path.join(kiln_repo.path, tarball_name)
    source_hashfile_name = f"{tarball_name}.sha256"
    source_hashfile_path = os.path.join(kiln_repo.path, hashfile_name)
    source_sig_name = f"{hashfile_name}.sig"
    source_sig_path = os.path.join(kiln_repo.path, sig_name)

    with io.BytesIO() as f:
        dulwich.porcelain.archive(kiln_repo, outstream=f)
        f.flush()
        compressed_bytes = lzma.compress(f.getvalue())
    with open(source_tarball_path, 'wb') as f:
        f.write(compressed_bytes)
    sha256sum = hashlib.sha256()
    sha256sum.update(compressed_bytes)
    tarball_hash = sha256sum.hexdigest()
    with open(source_hashfile_path, 'w') as f:
        f.write(f"{tarball_hash} {source_tarball_name}")

    with gpg.Context() as default_ctx:
        signing_key = default_ctx.get_key(signing_key_id)
        with gpg.Context(signers=[signing_key], armor=True) as ctx:
            with open(source_hashfile_path, 'rb') as hashfile:
                with open(source_sig_path, 'wb') as sigfile:
                    hashdata = hashfile.read()
                    sig, metadata = ctx.sign(hashdata, mode=gpg.constants.sig.mode.DETACH)
                    sigfile.write(sig)

    g = Github(github_personal_access_token)
    repo = g.get_repo("simplybusiness/Kiln")
    release = repo.create_git_release("v{version}", "Version {version}", changelog_lines.join('\n'), draft=True)
    release.upload_asset(tarball_path)
    release.upload_asset(hashfile_path)
    release.upload_asset(sig_path)
    release.upload_asset(source_tarball_path)
    release.upload_asset(source_hashfile_path)
    release.upload_asset(source_sig_path)


def docker_image_tags(version):
    tags = []
    if version.major == 0:
        tags.append(str(version))
        tags.append(f"{version.major}.{version.minor}")
        tags.append("latest")
    else:
        tags.append(str(version))
        tags.append(f"{version.major}.{version.minor}")
        tags.append(f"{version.major}")
        tags.append("latest")
    return tags


def set_cargo_toml_version(repo, component, version):
    with open(os.path.join(repo.path, component, "Cargo.toml"), "r+") as f:
        cargo_toml = toml.load(f)
        cargo_toml['package']['version'] = str(version)
        f.seek(0)
        f.truncate()
        toml.dump(cargo_toml, f, TomlPreserveInlineDictEncoder())


def set_kiln_lib_dependency(repo, component, sha=None, branch=None):
    with open(os.path.join(repo.path, component, "Cargo.toml"), "r+") as f:
        cargo_toml = toml.load(f)
        if sha is not None:
            try:
                del(cargo_toml['dependencies']['kiln_lib']['branch'])
            except KeyError:
                pass
            try:
                del(cargo_toml['dependencies']['kiln_lib']['rev'])
            except KeyError:
                pass
            cargo_toml['dependencies']['kiln_lib']['rev'] = sha.decode('utf-8')
        elif branch is not None:
            try:
                del(cargo_toml['dependencies']['kiln_lib']['branch'])
            except KeyError:
                pass
            try:
                del(cargo_toml['dependencies']['kiln_lib']['rev'])
            except KeyError:
                pass
            cargo_toml['dependencies']['kiln_lib']['branch'] = branch
        f.seek(0)
        f.truncate()
        toml.dump(cargo_toml, f, TomlPreserveInlineDictEncoder())


def check_for_expected_working_copy_changes(kiln_repo):
    (staged, unstaged, untracked) = dulwich.porcelain.status(kiln_repo)
    if staged['add'] or staged['delete'] or staged['modify']:
        return False
    for item in unstaged:
        if item != b"CHANGELOG.md" and not PurePath(item.decode("utf-8")).match("utils/*"):
            return False
    return True


if __name__ == "__main__":
    main()
