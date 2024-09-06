import os
import fnmatch

# Hardcoded values
ROOT_DIR = "."  # Current directory
OUTPUT_DIR = "outputs"  # Directory to store output files
EXCLUDE_PATTERNS = ["*target", "*git", "*idea", "output"]
FILE_EXTENSIONS = (".rs", ".toml", ".yaml", ".md")


def should_exclude(path):
    return any(fnmatch.fnmatch(os.path.basename(path), pattern) for pattern in EXCLUDE_PATTERNS)


def create_output_filename(dir_path):
    relative_path = os.path.relpath(dir_path, ROOT_DIR)
    return relative_path.replace(os.sep, "-") or "root"


def generate_content(file_path):
    with open(file_path, 'r', encoding='utf-8') as f:
        return f"# {os.path.relpath(file_path, ROOT_DIR)}\n```\n{f.read()}\n```\n\n"


def process_files():
    if not os.path.exists(OUTPUT_DIR):
        os.makedirs(OUTPUT_DIR)

    for root, dirs, files in os.walk(ROOT_DIR):
        dirs[:] = [d for d in dirs if not should_exclude(d)]

        dir_content = ""
        for file in files:
            if file.endswith(FILE_EXTENSIONS) and not should_exclude(file):
                file_path = os.path.join(root, file)
                dir_content += generate_content(file_path)

        if dir_content:
            output_filename = create_output_filename(root)
            output_path = os.path.join(OUTPUT_DIR, f"{output_filename}.txt")

            with open(output_path, 'w', encoding='utf-8') as f:
                f.write(dir_content)

            print(f"File '{output_path}' has been generated successfully.")


def main():
    process_files()
    print("All directories have been processed.")


if __name__ == "__main__":
    main()