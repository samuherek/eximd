release-cli args="":
    cargo release patch --no-publish -p cli {{args}}
