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
    "finstack/core/",
]

# Option to strip Rust comments (helps conserve tokens)
strip_rust_comments = True

# Define patterns to exclude
exclude_patterns = [
    "**/__pycache__",
    "**/__pycache__/**",
    "**/*.pyc",
    "**/*.pyo",
    "**/*.pyd",
    "**/*.so",
    "**/*.dylib",
    "**/*.dll",
    "**/*.wasm",
    "**/*.png",
    "**/*.jpg",
    "**/*.jpeg",
    "**/*.gif",
    "**/*.svg",
    "**/*.pdf",
    "**/*.zip",
    "**/*.tar",
    "**/*.gz",
    "**/*.bz2",
    "**/*.xz",
    "**/*.lock",
    "**/*.md",
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


def strip_rust_comments_from_text(content):
    """
    Remove Rust comments from text content.
    Handles single-line (//) and multi-line (/* */) comments.
    Note: This is a simple implementation that may not handle all edge cases
    (e.g., comment markers inside strings), but works well for most code.
    """
    import re
    
    # Remove multi-line comments /* ... */
    # This regex handles nested cases by being non-greedy
    content = re.sub(r'/\*.*?\*/', '', content, flags=re.DOTALL)
    
    # Remove single-line comments //
    # Only remove if // is not inside quotes (simple heuristic)
    lines = content.split('\n')
    cleaned_lines = []
    
    for line in lines:
        # Find // that's not in a string (simple check)
        comment_pos = line.find('//')
        if comment_pos != -1:
            # Simple heuristic: count quotes before the comment
            before_comment = line[:comment_pos]
            # If even number of quotes, likely not in string
            if before_comment.count('"') % 2 == 0 and before_comment.count("'") % 2 == 0:
                line = line[:comment_pos].rstrip()
        
        # Only keep non-empty lines or lines with actual content
        if line.strip():
            cleaned_lines.append(line)
    
    return '\n'.join(cleaned_lines)


def concatenate_files(output_filename="concatenated_code.txt"):
    """Concatenate all Python files and specified other files in the specified directories."""
    file_paths = []
    allowed_suffixes = {
        ".rs",
        ".toml",
        ".py",
        ".json",
        ".yml",
        ".yaml",
        ".sh",
        ".txt",
    }
    allowed_filenames = {"Makefile", "LICENSE", "Cargo.lock"}

    # If no includes specified, include everything at the top level
    paths_to_include = include_paths or get_default_includes()

    for path_str in paths_to_include:
        path = Path(path_str)
        if path.exists():
            # Only descend into dirs, or add the file directly
            if path.is_dir():
                # Skip heavy/binary build and vendor directories
                skip_dirs = {"target", "node_modules", "pkg", ".git", "__pycache__","docs","examples"}
                for candidate in path.rglob("*"):
                    # Fast skip for directories by name
                    try:
                        if candidate.is_dir() and candidate.name in skip_dirs:
                            continue
                    except Exception:
                        # Be resilient to permission or symlink errors
                        continue

                    if (
                        candidate.is_file()
                        and not is_excluded(candidate, exclude_patterns)
                        and (
                            candidate.suffix in allowed_suffixes
                            or candidate.name in allowed_filenames
                        )
                    ):
                        file_paths.append(candidate)
            elif path.is_file():
                # Include any file type, not just .py files
                if not is_excluded(path, exclude_patterns) and (
                    path.suffix in allowed_suffixes or path.name in allowed_filenames
                ):
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
                    
                    # Strip Rust comments if enabled and file is a .rs file
                    if strip_rust_comments and file_path.suffix == ".rs":
                        content = strip_rust_comments_from_text(content)
                    
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
            comment_status = " (with Rust comments stripped)" if strip_rust_comments else ""
            logger.info(
                f"Successfully concatenated {len(file_paths)} files into {output_filename}{comment_status}"
            )
    except Exception as e:
        logger.error("Error writing to output file %s: %s", output_filename, e)


if __name__ == "__main__":
    concatenate_files()
