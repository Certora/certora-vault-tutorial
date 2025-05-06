import argparse
import subprocess
import sys
from pathlib import Path
import logging
from string import Template
import shutil

logging.basicConfig(level=logging.INFO, format='%(message)s')

def install_certora_build(package_path, execute=True):
    source_path = Path('./scripts/certora_build.py')
    destination_path = package_path / 'certora_build.py'

    if not execute:
        logging.info(f"Copied {source_path} to {destination_path}")
        return


    if not source_path.exists():
        logging.error(f"Source file {source_path} does not exist.")
        sys.exit(1)

    try:
        shutil.copy(source_path, destination_path)
        logging.info(f"Copied {source_path} to {destination_path}")
    except Exception as e:
        logging.error(f"Failed to copy {source_path} to {destination_path}: {e}")
        sys.exit(1)

def make_package_justfile(workspace_relative_path, package_name, package_path, execute=True):
    # read content of './just/package-justfile.template as a Template
    template_path = Path('./just/package-justfile.template')
    if not template_path.exists():
        logging.error(f"Template file {template_path} does not exist.")
        sys.exit(1)

    with template_path.open('r') as template_file:
        template_content = Template(template_file.read())

    # substitute workspace_relative_path and package_name in the template
    justfile_content = template_content.substitute(
        relative_workspace_path=workspace_relative_path,
        package_name=package_name
    )

    # write the result to package_path / 'justfile'
    justfile_path = package_path / 'justfile'
    if execute:
        with justfile_path.open('w') as justfile:
            justfile.write(justfile_content)
    else:
        logging.debug(justfile_content)

    logging.info(f"Generated {justfile_path}")


def extend_file(file_path, extra_content, execute=True):
    if not execute:
        logging.info(f"Appended Certora content to {file_path}")
        return
    try:
        # save a copy
        saved_toml_file_path = file_path.with_suffix(file_path.suffix + '.orig')
        shutil.copy(file_path, saved_toml_file_path)
        with file_path.open('a') as toml_file:
            toml_file.write('\n\n\n')
            toml_file.write(extra_content.strip())
            toml_file.write('\n')
        logging.info(f"Appended Certora content to {file_path}")
    except Exception as e:
        logging.error(f"Failed to extend {file_path}: {e}")
        sys.exit(1)


def extend_workspace_toml(toml_file_path, execute=True):
    toml_extra_content = '''
# === Certora CVLR ===
[workspace.dependencies.cvlr]
version = "0.4.0"

[workspace.dependencies.cvlr-solana]
version = "0.4.4"
    '''
    extend_file(toml_file_path, toml_extra_content, execute=execute)


def extend_package_toml(toml_file_path, execute=True):
    toml_extra_content = '''
# === Certora CVLR ===
[dependencies.cvlr]
workspace = true
optional = true

[dependencies.cvlr-solana]
workspace = true
optional = true

[package.metadata.certora]
sources = [ "src/**/*.rs" ]
solana_inlining = [ "src/certora/envs/solana_inlining.txt" ]
solana_summaries = [ "src/certora/envs/solana_summaries.txt" ]
   '''
    extend_file(toml_file_path, toml_extra_content, execute=execute)

def extend_git_ignore(file_path, execute=True):
    git_ignore_extra_content = '''
.certora
.certora_internal
certora_out
'''
    extend_file(file_path, git_ignore_extra_content, execute=execute)

def main():
    parser = argparse.ArgumentParser(description="Certora setup script.")
    parser.add_argument('--workspace', type=Path,
                        required=True, help='Path to the workspace')
    parser.add_argument('--package', type=Path,
                        help='Path to the package', default="../../")
    parser.add_argument('--execute', action='store_true',
                        help='Execute the setup steps')
    parser.add_argument('--package-name', type=str,
                        required=True, help='Name of the package')

    args = parser.parse_args()

    workspace_path = args.workspace.resolve()
    package_path = args.package.resolve()

    workspace_toml = workspace_path / "Cargo.toml"
    package_toml = package_path / "Cargo.toml"

    if not workspace_toml.exists():
        logging.error(f"{workspace_toml}: does not exist.")
        sys.exit(1)

    if not package_toml.exists():
        logging.error(f"{package_toml}: does not exist.")
        sys.exit(1)

    workspace_relative_path = '../' * \
        len(package_path.relative_to(workspace_path).parts)

    make_package_justfile(workspace_relative_path,
                          args.package_name, package_path, execute=args.execute)
    extend_workspace_toml(workspace_toml, execute=args.execute)
    extend_package_toml(package_toml, execute=args.execute)
    install_certora_build(package_path, execute=args.execute)
    extend_git_ignore(workspace_path / '.gitignore', execute=args.execute)

    if args.execute:
        # delete .git directory
        git_dir = Path(".git")
        if git_dir.exists() and git_dir.is_dir():
            try:
                shutil.rmtree(git_dir)
                logging.info(f"Deleted {git_dir}")
            except Exception as e:
                logging.error(f"Failed to delete {git_dir}: {e}")
                sys.exit(1)
    else:
        logging.info(
            "Execution flag not set. No files have been changed. Run with --execute option.")


if __name__ == "__main__":
    main()
