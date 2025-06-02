# Formanator 🤖

Formanator allows you to **submit benefit claims to [Forma](https://www.joinforma.com/) from the command line**, either one-by-one or in bulk.

With the power of large language models 🧠👀 - free of charge thanks to [GitHub Models](https://docs.github.com/en/github-models/use-github-models/prototyping-with-ai-models) - it can even **analyse your receipts and generate your claims automatically**.

![Screenshot of running `formanator` from a terminal](https://github.com/user-attachments/assets/e053efc8-f4cb-4ea1-8850-6c82d668bf29)

# Installation

To install Formanator, make sure you have [Node.js](https://nodejs.org/en) installed, and then just run:

```bash
npm install -g formanator
```

To be able to automatically infer claim details from PDF receipts, you will also need to install Ghostscript and Graphicsmagick:

```bash
brew install ghostscript graphicsmagick
```

# Usage

## Connecting to your Forma account

To get started, you'll need to connect Formanator to your Forma account. Here's how the process works:

1. Run `formanator login`.
2. Provide your email in one of the three ways:
    1. At the prompt, enter your email address, then hit Enter.
    2. Set the `FORMA_EMAIL` environment variable
    3. Use the `--email` argument
3. You'll be sent an email with a magic link. Go to your inbox and copy the link to your clipboard.
4. At the prompt, paste your magic link, then hit Enter.
5. You'll be logged in 🥳

To remember your login, Formanator stores a `.formanator.json` file in your home directory with your access token.

## Configuring GitHub Models or OpenAI for inferring claim details

When submitting a claim, you need to specify several details like amount, merchant, purchase date, description, benefit and category. You can either input these manually, or use a large language model (LLM) to infer them.

### Using GitHub Models to infer claim details

[GitHub Models](https://github.blog/news-insights/product-news/introducing-github-models/) gives a generous free tier for various AI models, so you can do this totally free of charge. 

You'll just to configure a GitHub personal access token (PAT) with models access:

1. Create a [GitHub Token](https://github.com/settings/personal-access-tokens) with read access to GitHub Models.
2. Set the Token as the `GITHUB_TOKEN` environment variable, or be prepared to pass the `--github-token` argument to every command.

### Using OpenAI to infer claim detils

You can also use OpenAI's API to infer claim details. The cost is minimal, at $0.01-0.02 per receipt for full inference, or $0.001 for benefit/category only.

You'll need to configure an OpenAI API key:

1. Set up an OpenAI account and make sure you either (a) have free trial credit available or (b) have set up a payment method. You can check this on the ["Usage"](https://platform.openai.com/account/usage) page.
2. Create an [OpenAI API key](https://platform.openai.com/account/api-keys).
3. Set the API key as the `OPENAI_API_KEY` environment variable, or be prepared to pass the `--openai-api-key` argument to every command.

## Submitting claims in bulk (recommended)

### Automatically submitting all receipts in a directory (recommended)

You can submit all receipts in a specific directory, using a large language model (LLM) to infer the claim details for each receipt.

```bash
# You'll need to set GITHUB_TOKEN or OPENAI_API_KEY, or specify --github-token or --openai-api-key
formanator submit-claims-from-directory --directory input/
```

All JPG, PNG, PDF and HEIC receipts in the directory will be processed. The tool will allow you to confirm the details for each receipt before submitting.

### Manually submitting receipts using a CSV template

You can submit multiple claims at once by generating a template CSV, filling it in, then submitting the whole CSV. Optionally, the tool can infer the benefit and category for each claim.

1. Make sure you're logged in - for more details, see "Connecting to your Forma account" above.
2. Run `formanator generate-template-csv` to generate a CSV template. By default, the template will be saved as `claims.csv`. Optionally, you can specify the `--output-path` argument to choose where to save the template.
3. If you aren't using OpenAI to infer the benefit and category for each claim, you'll need to figure this out yourself. Get a list of your available benefits by running `formanator benefits`. Pick the relevant benefit, and then run `formanator categories --benefit <benefit>` to get a list of categories.
4. Update the template, filling in the columns for each of your claims. If you've configured OpenAI or GitHub Models, you can leave the `benefit` and `category` blank. If you want to attach multiple receipts, you can add comma-separated paths to the `receipt_path` column.
5. Validate the CSV up-front by running `formanator validate-csv --input-path claims.csv`.
6. Submit your claims by running `formanator submit-claims-from-csv --input-path claims.csv`.
7. If you've configured OpenAI, you'll be given the chance to check the benefit and category it has inferred for each claim.
8. Your claims will be submitted. If there are any validation errors with any of the rows, or if anything goes wrong during submission, an error message will be displayed, but the tool will continue submitting other claims.

## Submitting a single claim

You have several options for submitting a claim:

### Option 1: Infer all claim details from receipt (recommended)

If you have configured GitHub Models or OpenAI, you can simply provide a receipt image and let the model extract ALL the details.

```bash
# You'll need to set GITHUB_TOKEN or OPENAI_API_KEY, or specify --github-token or --openai-api-key
formanator submit-claim --receipt-path "receipt.jpg"
```

The LLM will analyze your receipt and extract:
- Amount
- Merchant name  
- Purchase date
- Description of items
- Appropriate benefit and category

You'll be shown the extracted details and asked to confirm before submitting.

**Supported receipt formats**: JPEG, PNG, PDF, and HEIC files (PDF requires GraphicsMagick and Ghostscript)

### Option 2: Infer beenfit and category from claim details

If you want to provide some details manually, but let the model infer the benefit and category:

```bash
# You'll need to set GITHUB_TOKEN or OPENAI_API_KEY, or specify --github-token or --openai-api-key
formanator submit-claim --amount 2.28 \
                        --merchant Amazon \
                        --description "USB cable" \
                        --purchase-date 2023-01-15 \
                        --receipt-path "USB.pdf"
```

You'll be given the chance to review the inferred details. If you confirm by hitting Enter, your claim will be submitted.

### Option 2: Manual entry

You can provide all claim details manually, with no LLM inference.

1. Figure out the benefit and category for your claim. Get a list of your available benefits by running `formanator benefits`. Pick the relevant benefit, and then run `formanator categories --benefit <benefit>` to get a list of categories.

2. Submit your claim by running `formanator submit-claim` with all required details:

```bash
formanator submit-claim --amount 2.28 \
                        --merchant Amazon \
                        --description "USB cable" \
                        --purchase-date 2023-01-15 \
                        --receipt-path "USB.pdf" \
                        --benefit "Remote Life" \
                        --category "Cables & Cords"
```


## Contributing

Changes to this project are verioned using [Semantic Versioning](https://semver.org/) and released to `npm` automatically using [`semantic-release`](https://github.com/semantic-release/semantic-release). 

Commit messages must follow [Angular Commit Message Conventions](https://github.com/angular/angular/blob/master/CONTRIBUTING.md#-commit-message-format) so `semantic-release` knows when to release new versions and what version number to use.
