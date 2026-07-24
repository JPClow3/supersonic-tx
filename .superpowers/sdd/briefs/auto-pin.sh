set +e
for i in $(seq 1 40); do
  out=$(cargo metadata --format-version 1 2>&1)
  ec=$?
  if [ $ec -eq 0 ]; then echo METADATA_OK iter=$i; exit 0; fi
  path=$(echo "$out" | grep -oP '/\K[a-zA-Z0-9_-]+-[0-9][^/]+(?=/Cargo.toml)' | head -1)
  pkg=$(echo "$path" | sed 's/-[0-9].*//')
  ver=$(echo "$path" | sed 's/^.*-//')
  echo "iter $i pin $pkg@$ver"
  patch=$(echo "$ver" | awk -F. '{print $NF}')
  base=$(echo "$ver" | sed 's/\.[0-9]*$//')
  if [ "$patch" -gt 0 ] 2>/dev/null; then
    newver="$base.$((patch-1))"
    cargo update -p "${pkg}@${ver}" --precise "$newver" 2>&1 | tail -2
  else
    echo "cannot auto pin $pkg $ver"; echo "$out" | tail -10; exit 1
  fi
done
exit 1