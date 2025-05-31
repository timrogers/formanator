# Formanator 🤖

Formanator allows you to submit benefit claims to [Forma](https://www.joinforma.com/) from the command line, either one-by-one or in bulk.

![Screenshot of running `formanator` from a terminal](https://github.com/timrogers/formanator/assets/116134/2979fda6-415c-4212-9263-7707841a03bf)

## Installation

To install Formanator, make sure you have [Node.js](https://nodejs.org/en) installed, and then just run:

```bash
npm install -g formanator
```

## Usage

### Connecting to your Forma account

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

### Configuring OpenAI for inferring claim details

When submitting a claim, you need to specify several details like amount, merchant, purchase date, description, benefit and category. You can either input these manually, or you can have OpenAI infer them from your receipt image using advanced vision models! 🤖👁️

OpenAI can help you in two ways:
1. **Full Receipt Inference** (recommended): Analyze a receipt image and extract ALL claim details automatically 
2. **Benefit/Category Inference**: Infer just the benefit and category based on merchant name and description you provide

The cost is minimal - about $0.01-0.02 per receipt for full inference, or $0.001 for benefit/category only.

If you want to use OpenAI to help with your claims, you'll need to set it up:

1. Set up an OpenAI account and make sure you either (a) have free trial credit available or (b) have set up a payment method. You can check this on the ["Usage"](https://platform.openai.com/account/usage) page.
2. Create an [OpenAI API key](https://platform.openai.com/account/api-keys).
3. Set the API key as the `OPENAI_API_KEY` environment variable, or be prepared to pass the `--openai-api-key` argument to every command.

### Configuring GitHub Models for inferring the benefit and category

[GitHub Models](https://github.blog/news-insights/product-news/introducing-github-models/) gives a generous free tier for various AI models. If you prefer, you can use GitHub Models instead of OpenAI directly.

To use GitHub Models to infer the benefit and category, you'll need to set it up.

1. Create a [GitHub Token](https://github.com/settings/personal-access-tokens) with read access to GitHub Models.
2. Set the Token as the `GITHUB_TOKEN` environment variable, or be prepared to pass the `--github-token` argument to every command.

### Submitting a single claim

You have several options for submitting a claim:

#### Option 1: Full Receipt Inference with OpenAI (Easiest!)

If you have configured OpenAI, you can simply provide a receipt image and let OpenAI extract ALL the details:

```bash
formanator submit-claim --receipt-path "receipt.jpg" --openai-api-key YOUR_API_KEY
# or if you've set OPENAI_API_KEY environment variable:
formanator submit-claim --receipt-path "receipt.jpg"
```

OpenAI will analyze your receipt and extract:
- Amount
- Merchant name  
- Purchase date
- Description of items
- Appropriate benefit and category

You'll be shown the extracted details and asked to confirm before submitting.

**Supported receipt formats**: JPEG, PNG, PDF, and HEIC files (PDF requires GraphicsMagick and Ghostscript)

#### Option 2: Manual Entry

1. Figure out what you're planning to claim for.
2. Make sure you're logged in - for more details, see "Connecting to your Forma account" above.
3. If you aren't using OpenAI, you'll need to figure out the benefit and category yourself. Get a list of your available benefits by running `formanator benefits`. Pick the relevant benefit, and then run `formanator categories --benefit <benefit>` to get a list of categories.
4. Submit your claim by running `formanator submit-claim` with all required details:

```bash
formanator submit-claim --amount 2.28 \
                        --merchant Amazon \
                        --description "USB cable" \
                        --purchase-date 2023-01-15 \
                        --receipt-path "USB.pdf" \
                        --benefit "Remote Life" \
                        --category "Cables & Cords"
```

#### Option 3: Partial OpenAI Assistance

If you want to provide some details manually but let OpenAI infer the benefit and category:

```bash
formanator submit-claim --amount 2.28 \
                        --merchant Amazon \
                        --description "USB cable" \
                        --purchase-date 2023-01-15 \
                        --receipt-path "USB.pdf" \
                        --openai-api-key YOUR_API_KEY
```

5. If you've configured OpenAI, you'll be given the chance to review the inferred details.
6. If you confirm by hitting Enter, your claim will be submitted.

### Submitting multiple claims

You can submit multiple claims at once using two approaches:

#### Option 1: Process receipts from a directory (recommended)

If you have a directory full of receipt images, you can process them all at once with full receipt inference:

```bash
formanator submit-receipts-from-directory --directory ./receipts --openai-api-key YOUR_API_KEY
# or if you've set OPENAI_API_KEY environment variable:
formanator submit-receipts-from-directory --directory ./receipts
```

This command will:
1. Find all supported receipt files (.jpg, .jpeg, .png, .pdf, .heic) in the specified directory
2. Use OpenAI to analyze each receipt and extract all claim details
3. Show you the extracted details for each receipt and ask for confirmation (Y/N)
4. Submit approved claims to Forma
5. Move successfully processed receipts to a `processed/` subdirectory (ensuring idempotence)

**Options:**
- `--directory`: The directory containing receipt files (required)
- `--processed-directory`: Custom directory for processed receipts (defaults to `processed/` subdirectory)
- `--openai-api-key` or `--github-token`: API key for receipt inference (required)

**Example with custom processed directory:**
```bash
formanator submit-receipts-from-directory --directory ./receipts --processed-directory ./completed
```

When you run the command again, it will only process receipts that haven't been moved to the processed directory yet.

#### Option 2: CSV workflow

You can also submit multiple claims using a CSV template (supports benefit/category inference but not full receipt inference):

**Note**: The CSV workflow currently supports benefit/category inference but not full receipt inference. For full receipt inference, use the directory processing approach above or submit claims individually.

1. Make sure you're logged in - for more details, see "Connecting to your Forma account" above.
2. Run `formanator generate-template-csv` to generate a CSV template. By default, the template will be saved as `claims.csv`. Optionally, you can specify the `--output-path` argument to choose where to save the template.
3. If you aren't using OpenAI to infer the benefit and category for each claim, you'll need to figure this out yourself. Get a list of your available benefits by running `formanator benefits`. Pick the relevant benefit, and then run `formanator categories --benefit <benefit>` to get a list of categories.
4. Update the template, filling in the columns for each of your claims. If you've configured OpenAI, you can leave the `benefit` and `category` blank. If you want to attach multiple receipts, you can add comma-separated paths to the `receipt_path` column.
5. Validate the CSV up-front by running `formanator validate-csv --input-path claims.csv`.
6. Submit your claims by running `formanator submit-claims-from-csv --input-path claims.csv`.
7. If you've configured OpenAI, you'll be given the chance to check the benefit and category it has inferred for each claim.
8. Your claims will be submitted. If there are any validation errors with any of the rows, or if anything goes wrong during submission, an error message will be displayed, but the tool will continue submitting other claims.

## Contributing

Changes to this project are verioned using [Semantic Versioning](https://semver.org/) and released to `npm` automatically using [`semantic-release`](https://github.com/semantic-release/semantic-release). 

Commit messages must follow [Angular Commit Message Conventions](https://github.com/angular/angular/blob/master/CONTRIBUTING.md#-commit-message-format) so `semantic-release` knows when to release new versions and what version number to use.
