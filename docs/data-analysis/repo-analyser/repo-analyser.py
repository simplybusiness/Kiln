from pygit2 import discover_repository, Repository
from pygit2 import GIT_SORT_REVERSE, GIT_SORT_TIME, GIT_CHECKOUT_FORCE, GIT_CHECKOUT_RECREATE_MISSING
import argparse
import os
import subprocess
import itertools

parser = argparse.ArgumentParser(description='Run Kiln for every commit on a master branch in a git repo')
parser.add_argument('dir')
parser.add_argument('app_name')
parser.add_argument('data_collector_url')
args = parser.parse_args()
proj_dir = vars(args)['dir']
app_name = vars(args)['app_name']
url = vars(args)['data_collector_url']

repo_path = discover_repository(proj_dir)
if repo_path == None:
    raise Exception("Project directory isn't a git repo. Exiting!")

repo = Repository(repo_path)
walker = repo.walk(repo.branches['master'].peel().id, GIT_SORT_REVERSE | GIT_SORT_TIME)
walker.simplify_first_parent()
all_commits = [x.id for x in walker]
kiln_config_path = os.path.abspath(os.path.join(proj_dir, "kiln.toml"))
for commit in all_commits:
    if commit == "":
        continue
    subprocess.check_call(["git", "reset", "--hard", "HEAD"], cwd=proj_dir, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    subprocess.check_call(["git", "checkout", str(commit)], cwd=proj_dir, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

    if not os.path.exists(os.path.abspath(os.path.join(proj_dir, "Gemfile.lock"))):
        print("No Gemfile.lock, skipping commit ", str(commit))
        continue

    with open(kiln_config_path, 'w') as f:
        f.truncate(0)
        f.writelines([f'app_name="{app_name}"\n', 'data_collector_url="{url}"'])
        f.flush()
    try:
        subprocess.check_output(["kiln-cli", "--use-local-image", "ruby", "dependencies"], cwd=proj_dir)
    except subprocess.CalledProcessError as err:
        print("Something went wrong when running Kiln on commit ", rev.id)
        raise
