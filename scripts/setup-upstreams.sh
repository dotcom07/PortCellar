#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

git submodule sync --recursive

top_level_paths=(
  upstreams/FAudio
  upstreams/MoltenVK
  upstreams/dxvk
  upstreams/gstreamer
  upstreams/macports-wine
  upstreams/scgl
  upstreams/vkd3d
  upstreams/wine
)

git submodule update --init -- "${top_level_paths[@]}"
git submodule update --init --recursive upstreams/dxvk
