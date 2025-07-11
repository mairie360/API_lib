#!/usr/bin/env bash
set -e

VERSION=$1
if [ -z "$VERSION" ]; then
    echo "Usage: ./publish.sh <version>"
    exit 1
fi

echo "🚀 Publishing $VERSION to private cargo registry"

if [ -z "$MAIRIE_360_DEPLOY_TOKEN" ]; then
    echo "Error: MAIRIE_360_DEPLOY_TOKEN environment variable is not set"
    exit 1
fi

# 🟢 récupère le nom de la crate depuis ton projet
CRATE_NAME=$(cargo metadata --no-deps --format-version=1 | jq -r '.packages[0].name')
echo "Crate name: $CRATE_NAME"

cargo package

CRATE_FILE="target/package/${CRATE_NAME}-${VERSION}.crate"
if [ ! -f "$CRATE_FILE" ]; then
    echo "Error: crate file $CRATE_FILE does not exist"
    exit 1
fi

CHECKSUM=$(sha256sum "$CRATE_FILE" | cut -d ' ' -f1)

DEPS_JSON=$(cargo metadata --format-version=1 --no-deps | jq --arg CRATE "$CRATE_NAME" '.packages[] | select(.name == $CRATE) | .dependencies | map({
  name: .name,
  req: .req,
  features: [],
  optional: false,
  default_features: true,
  target: null,
  kind: "normal",
  registry: null,
  package: null
})')

# 🔵 clone l'index APRÈS avoir trouvé les infos
git clone https://$MAIRIE_360_DEPLOY_TOKEN@github.com/mairie360/cargo-index.git /tmp/index

FIRST_CHAR=$(echo -n "$CRATE_NAME" | head -c 1)
CRATE_INDEX_FILE="/tmp/index/$FIRST_CHAR/$CRATE_NAME"

mkdir -p "$(dirname "$CRATE_INDEX_FILE")"

# Prépare la nouvelle ligne JSON pour cette version
NEW_LINE=$(jq -nc --arg name "$CRATE_NAME" --arg vers "$VERSION" --argjson deps "$DEPS_JSON" --arg cksum "$CHECKSUM" \
    '{name: $name, vers: $vers, deps: $deps, cksum: $cksum, features: {}, yanked: false, links: null}')

# Si le fichier existe déjà, on filtre les versions différentes pour éviter doublons
if [ -f "$CRATE_INDEX_FILE" ]; then
    # Supprime la ligne pour cette version si elle existe déjà
    grep -v "\"vers\":\"$VERSION\"" "$CRATE_INDEX_FILE" > "$CRATE_INDEX_FILE.tmp" || true
    mv "$CRATE_INDEX_FILE.tmp" "$CRATE_INDEX_FILE"
fi

# Ajoute la nouvelle version à la fin
echo "$NEW_LINE" >> "$CRATE_INDEX_FILE"

cd /tmp/index
git add "$CRATE_INDEX_FILE"
git commit -m "Add $CRATE_NAME $VERSION"
git push

# echo "📤 Uploading crate"
# scp "$CRATE_FILE" user@mairie360-eip.fr:/var/www/html/crates/$CRATE_NAME/$VERSION/download
