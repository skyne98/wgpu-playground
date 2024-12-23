import os
import json
import re

def get_project_directories(base_path):
    """Get directories that start with a number."""
    return [
        d for d in os.listdir(base_path)
        if os.path.isdir(os.path.join(base_path, d)) and re.match(r'^\d', d)
    ]

def collect_source_files(project_path):
    """Recursively collect all source files from the project directory."""
    source_files = {}
    for root, _, files in os.walk(project_path):
        for file in files:
            if file.endswith((".rs", ".glsl", ".wgsl", ".h", ".cpp", ".py", ".js", ".ts")):
                relative_path = os.path.relpath(os.path.join(root, file), project_path)
                with open(os.path.join(root, file), 'r', encoding='utf-8') as f:
                    source_files[relative_path] = f.read()
    return source_files

def create_prompt(source_files):
    """Create a structured prompt from collected source files."""
    prompt = ["\"\"\""]
    for file_path, content in source_files.items():
        prompt.append(f"```\n### File: {file_path}\n\n{content}\n```\n")
    prompt.append("\"\"\"")
    return "\n".join(prompt)

def main():
    base_path = os.path.abspath(os.path.join(os.path.dirname(__file__), '..'))
    projects = get_project_directories(base_path)

    print("Available projects:")
    for i, project in enumerate(projects):
        print(f"{i}: {project}")

    try:
        choice = int(input("Enter the number of the project you want to pack: "))
        if choice < 0 or choice >= len(projects):
            print("Invalid choice.")
            return
    except ValueError:
        print("Invalid input. Please enter a number.")
        return

    selected_project = projects[choice]
    project_path = os.path.join(base_path, selected_project)

    print(f"Packing source files from project: {selected_project}")
    source_files = collect_source_files(project_path)
    prompt = create_prompt(source_files)

    output_file = os.path.join(base_path, 'scripts', f"{selected_project}_prompt.txt")
    with open(output_file, 'w', encoding='utf-8') as f:
        f.write(prompt)

    print(f"Prompt written to {output_file}")

if __name__ == "__main__":
    main()
