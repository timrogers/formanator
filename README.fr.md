# Formanator 🤖

> [!NOTE]
> 🦀 Formanator est maintenant construit avec Rust et distribué via Homebrew et Crates.io ! [v2.x](https://github.com/timrogers/formanator/releases/tag/v2.2.0), construit en TypeScript et distribué via npm, est [toujours disponible](https://github.com/timrogers/formanator/releases/tag/v2.2.0).

Formanator vous permet de :

* **Soumettre des demandes de prestations à [Forma](https://www.joinforma.com/) et suivre la progression depuis la ligne de commande**, une par une ou en masse
* **Comprendre vos prestations Forma et suivre et soumettre des demandes depuis n'importe quel client Model Context Protocol (MCP)**, par exemple [Copilot CLI](https://github.com/features/copilot/cli), [Visual Studio Code](https://code.visualstudio.com/) ou [Claude Code](https://code.claude.com/docs/en/overview)

Avec la puissance des grands modèles de langage 🧠👀 - via la [GitHub Copilot CLI](https://github.com/features/copilot/cli) (gratuit, aucune configuration supplémentaire nécessaire) ou [OpenAI](https://openai.com/) - il peut même **analyser vos reçus et générer vos demandes automatiquement**.

![Capture d'écran de l'exécution de `formanator` à partir d'un terminal](https://github.com/user-attachments/assets/e053efc8-f4cb-4ea1-8850-6c82d668bf29)

## Installation

### macOS ou Linux via [Homebrew](https://brew.sh/)

```bash
brew tap timrogers/tap && brew install formanator
```

### macOS, Linux ou Windows via [Cargo](https://doc.rust-lang.org/cargo/), le gestionnaire de paquets de Rust

1. Installez [Rust](https://www.rust-lang.org/tools/install) sur votre machine, s'il n'est pas déjà installé.
1. Installez la caisse `formanator` en exécutant `cargo install formanator`.
1. Exécutez `formanator --help` pour vérifier que tout fonctionne et voir les commandes disponibles.

### macOS, Linux ou Windows via téléchargement direct de binaire

1. Téléchargez la [dernière version](https://github.com/timrogers/formanator/releases/latest) pour votre plateforme. Les appareils macOS, Linux et Windows sont pris en charge.
1. Ajoutez le binaire à votre `PATH` (ou `$PATH` sur les systèmes de type Unix), afin que vous puissiez l'exécuter à partir de votre shell/terminal. Pour la meilleure expérience, nommez-le `formanator` (ou `formanator.exe` sur Windows).
1. Exécutez `formanator --help` pour vérifier que tout fonctionne.

### À partir de la source

```bash
git clone https://github.com/timrogers/formanator
cd formanator
cargo install --path .
```

### Optionnel : Support des reçus PDF

Pour déduire les détails de la demande pour les reçus PDF, vous devez avoir [GraphicsMagick](http://www.graphicsmagick.org/) et [Ghostscript](https://www.ghostscript.com/) installés.

```bash
# macOS
brew install graphicsmagick ghostscript
```

## Utilisation

### Connexion à votre compte Forma

Pour commencer, vous devez connecter Formanator à votre compte Forma :

1. Exécutez `formanator login`.
2. Appuyez sur Entrée pour ouvrir votre navigateur à la page de connexion Forma.
3. Entrez votre adresse e-mail et demandez un lien magique.
4. Copiez le lien magique de votre e-mail et collez-le dans le terminal.
5. Vous êtes connecté 🥳

Le jeton d'accès est stocké de manière sécurisée dans le Keychain système sur macOS. Sur les autres plates-formes, il est stocké dans `~/.formanator.toml`.

### Vérifications automatiques des mises à jour

Une fois par jour, Formanator vérifie GitHub pour une version plus récente. Quand une est disponible, il imprime un avis jaune sur stderr avant d'exécuter votre commande. La vérification est limitée en enregistrant le dernier timestamp de vérification dans `~/.formanator.toml`, considère uniquement les versions d'au moins 72 heures, et expire après 2 secondes pour ne pas ralentir la CLI. Pour désactiver complètement la vérification, définissez la variable d'environnement `FORMANATOR_DISABLE_UPDATE_CHECK` sur une valeur quelconque.

### Configuration d'un fournisseur LLM (optionnel, mais recommandé)

Lors de la soumission d'une demande, vous pouvez soit fournir chaque détail manuellement, soit laisser un LLM les déduire. Deux fournisseurs sont pris en charge :

- **GitHub Copilot CLI** — _la valeur par défaut._ Si vous ne configurez pas OpenAI, Formanator utilise la [GitHub Copilot CLI](https://github.com/features/copilot/cli) pour l'inférence. Formanator détecte automatiquement le binaire `copilot` sur votre `PATH` ; s'il se trouve ailleurs, définissez la variable d'environnement `COPILOT_CLI_PATH` ou passez `--copilot-cli-path` avec le chemin du binaire.
- **OpenAI** — facturé à votre compte OpenAI. Définissez la variable d'environnement `OPENAI_API_KEY`, ou passez `--openai-api-key`.

Si les deux sont configurés, Formanator préfère OpenAI et revient sinon à la GitHub Copilot CLI.

### Soumission des demandes en masse

#### Soumission automatique de tous les reçus dans un répertoire (recommandé)

```bash
formanator submit-claims-from-directory --directory input/
```

Tous les reçus `.jpg`, `.jpeg`, `.png`, `.pdf` et `.heic` du répertoire seront analysés par le LLM. Vous serez invité à confirmer les détails de la demande déduits pour chaque reçu avant sa soumission, et les reçus correctement soumis sont déplacés dans un sous-répertoire `processed/`.

#### Soumission manuelle de reçus à l'aide d'un modèle CSV

1. Générez un modèle : `formanator generate-template-csv` (écrit `claims.csv`).
2. Remplissez une ligne par demande. Si vous avez configuré un LLM, vous pouvez laisser `benefit` et `category` vides pour les faire déduire des autres champs, ou laisser chaque colonne vide sauf `receiptPath` pour faire déduire tous les détails de la demande à partir du reçu. Séparez par des virgules les chemins dans la colonne `receiptPath` pour joindre plusieurs reçus.
3. Validez optionnellement à l'avance : `formanator validate-csv --input-path claims.csv`.
4. Soumettez : `formanator submit-claims-from-csv --input-path claims.csv`.

### Soumission d'une seule demande

#### Option 1 : Déduire tous les détails de la demande à partir du reçu (recommandé)

```bash
formanator submit-claim --receipt-path receipt.jpg
```

Formanator demandera au LLM d'extraire le montant, le commerçant, la date d'achat, la description, la prestation et la catégorie, vous montrera le résultat et vous demandera de confirmer avant de soumettre.

#### Option 2 : Fournir les détails manuellement, déduire la prestation et la catégorie

```bash
formanator submit-claim \
  --amount 2.28 \
  --merchant Amazon \
  --description "USB cable" \
  --purchase-date 2024-01-15 \
  --receipt-path USB.pdf
```

#### Option 3 : Fournir chaque détail manuellement

```bash
formanator submit-claim \
  --amount 2.28 \
  --merchant Amazon \
  --description "USB cable" \
  --purchase-date 2024-01-15 \
  --receipt-path USB.pdf \
  --benefit "Remote Life" \
  --category "Cables & Cords"
```

Utilisez `formanator benefits` et `formanator categories --benefit <benefit>` pour découvrir les valeurs valides.

### Lister les demandes

```bash
formanator list-claims
formanator list-claims --filter in_progress
```

## Utilisation du Model Context Protocol (MCP)

Formanator peut s'exécuter en tant que serveur MCP sur stdio afin que les assistants IA puissent interagir avec votre compte Forma de manière programmatique.

```jsonc
{
  "mcpServers": {
    "formanator": {
      "command": "/path/to/formanator",
      "args": ["mcp"]
    }
  }
}
```

Le serveur expose six outils :

- `auth_status` — vérifier si Formanator est connecté à Forma.
- `login_start` — demander un lien magique Forma par e-mail.
- `login_complete` — terminer la connexion en collant le lien magique envoyé par e-mail.
- `list_benefits_with_categories` — lister toutes les prestations avec leurs catégories et soldes restants.
- `list_claims` — lister les demandes, avec filtrage optionnel (actuellement uniquement `in_progress`).
- `create_claim` — créer une nouvelle demande.

Si vous n'êtes pas déjà connecté, utilisez `login_start` et `login_complete` depuis votre client MCP. Vous pouvez toujours vous connecter à l'avance avec `formanator login`.

Pour construire un binaire sans support MCP (binaire plus petit, moins de dépendances) :

```bash
cargo install formanator --no-default-features
```

## Développement

```bash
cargo build              # construire avec les fonctionnalités par défaut (CLI + MCP)
cargo test --all-features
cargo clippy --all-features --all-targets -- -D warnings
cargo fmt --all
```
