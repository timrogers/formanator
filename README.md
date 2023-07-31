# Formanator 🤖

Formanator allows you to submit benefit claims to [Forma](https://www.joinforma.com/) from the command line, either one-by-one or in bulk.

![ray-so-export (1)](https://github.com/timrogers/formanator/assets/116134/880087c1-c9f2-479c-9993-1be2e3d368aa)

## Installation

To install Formanator, make sure you have [Node.js](https://nodejs.org/en) installed, and then just run:

```bash
npm install -g formanator
```

## Usage

### Connecting to your Forma account

To get started, you'll need to connect Formanator to your Forma account. Here's how the process works:

1. Run `formanator login`.
2. At the prompt, enter your email address, then hit Enter.
3. You'll be sent an email with a magic link. Go to your inbox and copy the link to your clipboard.
4. At the prompt, paste your magic link, then hit Enter.
5. You'll be logged in 🥳

To remember your login, Formanator stores a `.formanator.json` file in your home directory with your access token.

### Submitting a single claim

You can quickly submit a single claim from the command line.

When submitting a claim, you need to specify a benefit and category for your claim. You can either decide that yourself, or you can have OpenAI do it for you, for a cost of about $0.001 (a tenth of a cent! 🪙) per claim 🧠

#### Submitting a single claim with AI magic

1. Set up an OpenAI account and make sure you either (a) have free trial credit available or (b) have set up a payment method. You can check this on the ["Usage"](https://platform.openai.com/account/usage) page.
2. Create an [OpenAI API key](https://platform.openai.com/account/api-keys). Set the API key as the `OPENAI_API_KEY` environment variable.
3. Figure out what you're planning to claim for.
4. Make sure you're logged in - for more details, see "Connecting to your Forma account" above.
5. Submit your claim by running `formanator submit-claim`. You'll need to pass a bunch of arguments:

```bash
formanator submit-claim --amount 2.28 \
                        --merchant Amazon \
                        --description "USB cable" \
                        --purchase-date 2023-01-15 \
                        --receipt-path "USB.pdf"
```

6. We'll use OpenAI to figure out the benefit and category for your claim, and give you the chance to check the result.
7. If you confirm the benefit and category by hitting Enter, your claim will be submitted.

#### Submitting a single claim, specifying the benefit and category yourself

1. Figure out what you're planning to claim for.
2. Make sure you're logged in - for more details, see "Connecting to your Forma account" above.
3. Get a list of your available benefits by running `formanator benefits`. 
4. Pick the relevant benefit, and then run `formanator categories --benefit <benefit>` to get a list of categories. For example, running `formanator categories --benefit "Learning"` would return the applicable categories for the "Learning" benefit.
5. Submit your claim by running `formanator submit-claim`. You'll need to pass a bunch of arguments:

```bash
formanator submit-claim --benefit "Remote Life" \
                        --category "Cables & Cords" \
                        --amount 2.28 \
                        --merchant Amazon \
                        --description "USB cable" \
                        --purchase-date 2023-01-15 \
                        --receipt-path "USB.pdf"
```

6. Your claim will be submitted.

### Submiting multiple claims

You can submit multiple claims at once by generating a template CSV, filling it in, then submitting the whole CSV.

1. Make sure you're logged in - for more details, see "Connecting to your Forma account" above.
2. Run `formanator generate-template-csv` to generate a CSV template. By default, the template will be saved as `claims.csv`. Optionally, you can specify the `--output-path` argument to choose where to save the template.
3. Update the template, filling in the columns for each of your claims. To get valid `benefit` and `category` values, use the `formanator benefits` and `formanator categories --benefit <benefit>` commands documented in "Submitting a single claim, specifying the benefit and category yourself" above.
4. Submit your claims by running `formanator submit-claims-from-csv --input-path claims.csv`.
5. Your claims will be submitted. If there are any validation errors with any of the rows, or if anything goes wrong during submission, an error message will be displayed, but the tool will continue submitting other claims.

## Contributing

Changes to this project are verioned using [Semantic Versioning](https://semver.org/) and released to `npm` automatically using [`semantic-release`](https://github.com/semantic-release/semantic-release). 

Commit messages must follow [Angular Commit Message Conventions](https://github.com/angular/angular/blob/master/CONTRIBUTING.md#-commit-message-format) so `semantic-release` knows when to release new versions and what version number to use.
