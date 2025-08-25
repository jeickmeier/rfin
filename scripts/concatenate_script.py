# mypy: ignore-errors

"""Script to concatenate all source files for documentation purposes."""

import logging
from pathlib import Path

# Configure logging
logging.basicConfig(
    level=logging.INFO, format="%(asctime)s - %(name)s - %(levelname)s - %(message)s"
)
logger = logging.getLogger(__name__)

# Define the paths to include (can be empty for 'include everything' mode)
include_paths = [
    "docs/new",
]

# Define patterns to exclude
exclude_patterns = [
    "**/__pycache__",
]


def get_default_includes():
    """Return all dirs and files at top level if includes list is empty."""
    return [str(p) for p in Path().iterdir() if p.is_dir() or p.is_file()]


def is_excluded(path, patterns):
    """Check if a path matches any exclude pattern (very basic globbing)."""
    from fnmatch import fnmatch

    for pattern in patterns:
        # Match either by name (for dirs) or glob (for files)
        if fnmatch(path.name, pattern) or fnmatch(str(path), pattern):
            return True
    return False


def concatenate_files(output_filename="concatenated_code.txt"):
    """Concatenate all Python files and specified other files in the specified directories."""
    file_paths = []

    # If no includes specified, include everything at the top level
    paths_to_include = include_paths or get_default_includes()

    for path_str in paths_to_include:
        path = Path(path_str)
        if path.exists():
            # Only descend into dirs, or add the file directly
            if path.is_dir():
                for py_file in path.rglob("*.md"):
                    if not is_excluded(py_file, exclude_patterns):
                        file_paths.append(py_file)
            elif path.is_file():
                # Include any file type, not just .py files
                if not is_excluded(path, exclude_patterns):
                    file_paths.append(path)

    file_paths.sort()

    with open(output_filename, "w", encoding="utf-8") as output_file:
        for file_path in file_paths:
            output_file.write(f"\n{'=' * 80}\n")
            output_file.write(f"File: {file_path}\n")
            output_file.write(f"{'=' * 80}\n\n")

            try:
                with open(file_path, encoding="utf-8") as input_file:
                    content = input_file.read()
                    output_file.write(content)
                    output_file.write("\n\n")
            except FileNotFoundError:
                logger.warning("File not found at %s", file_path.resolve())
            except UnicodeDecodeError:
                logger.warning("Could not decode file %s as UTF-8; skipping", file_path)
            except Exception as e:
                logger.warning("Error reading file %s: %s", file_path, e)

    try:
        output_path = Path(output_filename)
        if output_path.exists():
            logger.info(
                f"Successfully concatenated {len(file_paths)} files into {output_filename}"
            )
    except Exception as e:
        logger.error("Error writing to output file %s: %s", output_filename, e)


if __name__ == "__main__":
    concatenate_files()
