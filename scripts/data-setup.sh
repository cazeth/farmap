#!/bin/bash

# Set default directory
default_dir="$HOME/.local/share/farmap"

# Prompt user for custom directory
read -p "Enter base directory (default: $default_dir): " input_dir
base_dir="${input_dir:-$default_dir}"

# Check if directory exists
if [ ! -d "$base_dir" ]; then
	read -p "Directory $base_dir does not exist. Create it? [y/N] " yn
	case $yn in
	[Yy]*) mkdir -p "$base_dir" || {
		echo "Failed to create directory"
		exit 1
	} ;;
	*)
		echo "Aborting."
		exit 1
		;;
	esac
fi

# Set repository path
repo_dir="$base_dir/labels"
mkdir -p "$repo_dir"

# Clone repository if it doesn't exist
if [ ! -d "$repo_dir/.git" ]; then
	echo "Cloning repository..."
	git clone https://github.com/warpcast/labels.git "$repo_dir" || {
		echo "Clone failed"
		exit 1
	}
else
	echo "Repository already exists at $repo_dir"
fi

# Process commits
(
	cd "$repo_dir" || exit 1
	echo "Processing commits..."

	# Process historical commits
	git log --reverse --pretty=format:"%H %aI" | while read -r commit_hash date_iso; do
		# Extract date from ISO format
		date=$(echo "$date_iso" | cut -d'T' -f1)
		filename="$base_dir/spam_$date.jsonl"

		# Check if spam.jsonl exists in this commit
		if git ls-tree --name-only -r "$commit_hash" | grep -q '^spam\.jsonl$'; then
			echo "Creating $filename from commit $commit_hash"
			git show "$commit_hash:spam.jsonl" >"$filename"
		fi
	done

	# Process the latest commit
	latest_commit_hash=$(git rev-parse HEAD)
	latest_date_iso=$(git log -1 --pretty=format:"%aI")
	latest_date=$(echo "$latest_date_iso" | cut -d'T' -f1)
	latest_filename="$base_dir/spam_$latest_date.jsonl"

	if git ls-tree --name-only -r "$latest_commit_hash" | grep -q '^spam\.jsonl$'; then
		echo "Creating $latest_filename from latest commit $latest_commit_hash"
		git show "$latest_commit_hash:spam.jsonl" >"$latest_filename"
	else
		echo "Warning: spam.jsonl not found in the latest commit"
	fi
)
echo "Processing complete. Files saved to $base_dir"
