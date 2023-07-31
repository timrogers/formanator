import * as commander from 'commander';
import chalk from 'chalk';
import { Configuration, OpenAIApi } from 'openai';
import { actionRunner, prompt } from '../utils.js';
import { getAccessToken } from '../config.js';
import { createClaim, getBenefitsWithCategories, } from '../forma.js';
import { claimParamsToCreateClaimOptions } from '../claims.js';
import VERSION from '../version.js';
const command = new commander.Command();
const generateOpenaiPrompt = (opts) => {
    const { description, merchant, validCategories } = opts;
    return `Your job is to predict the category for an expense claim based on the name of the merchant and a description of what was purchased. You should give a single, specific answer without any extra words or punctuation.

Here are the possible categories:

${validCategories.join('\n')}

Please predict the category for the following claim:

Merchant: ${merchant}
Description: ${description}`;
};
const attemptToinferCategoryAndBenefit = async (opts) => {
    const { merchant, description, benefitsWithCategories, openaiApiKey: apiKey } = opts;
    const configuration = new Configuration({
        apiKey,
    });
    const openai = new OpenAIApi(configuration);
    const categoriesWithBenefits = benefitsWithCategories.flatMap((benefit) => benefit.categories.map((category) => ({ ...category, benefit })));
    const validCategories = categoriesWithBenefits.flatMap((category) => category.subcategory_alias ?? category.subcategory_name);
    const content = generateOpenaiPrompt({ validCategories, merchant, description });
    const chatCompletion = await openai.createChatCompletion({
        model: 'gpt-3.5-turbo',
        messages: [
            {
                role: 'user',
                content,
            },
        ],
    });
    const returnedCategoryAsString = chatCompletion.data.choices[0].message?.content;
    if (!returnedCategoryAsString) {
        throw new Error(`Something went wrong while inferring the benefit and category for your claim. OpenAI returned an unexpected response: ${JSON.stringify(chatCompletion.data)}`);
    }
    const returnedCategory = categoriesWithBenefits.find((category) => category.subcategory_alias === returnedCategoryAsString ||
        category.subcategory_name === returnedCategoryAsString);
    if (!returnedCategory) {
        throw new Error(`Something went wrong while inferring the benefit and category for your claim. OpenAI returned a response that wasn't a valid category: ${returnedCategoryAsString}`);
    }
    console.log(`OpenAI inferred that you should claim using the ${chalk.magenta(returnedCategory.benefit.name)} benefit and ${chalk.magenta(returnedCategoryAsString)} category. If that seems right, hit Enter. If not, press Ctrl + C to end your session.`);
    prompt('> ');
    return { category: returnedCategoryAsString, benefit: returnedCategory.benefit.name };
};
command
    .name('submit-claim')
    .version(VERSION)
    .description('Submit a claim for a Forma benefit')
    .option('--benefit <benefit>', 'The benefit you are claiming for. You may omit this if an OpenAI API key is configured')
    .requiredOption('--amount <amount>', 'The amount of the claim')
    .requiredOption('--merchant <merchant>', 'The name of the merchant')
    .option('--category <category>', 'The category of the claim. You may omit this if an OpenAI API key is configured.')
    .requiredOption('--purchase-date <purchase-date>', 'The date of the purchase in YYYY-MM-DD format')
    .requiredOption('--description <description>', 'The description of the claim')
    .requiredOption('--receipt-path <receipt-path>', 'The path of the receipt. JPEG, PNG, PDF and HEIC files up to 10MB are accepted.')
    .option('--access-token <access_token>', 'Access token used to authenticate with Forma')
    .option('--openai-api-key <openai_token>', 'An optional OpenAI API key used to infer the benefit and category based on the merchant and description. If this is set, you may omit the `--benefit` and `--category` options. This can also be configured using the `OPENAI_API_KEY` environment variable.', process.env.OPENAI_API_KEY)
    .action(actionRunner(async (opts) => {
    const { benefit, category, openaiApiKey } = opts;
    const accessToken = opts.accessToken ?? getAccessToken();
    if (!accessToken) {
        throw new Error("You aren't logged in to Forma. Please run `formanator login` first.");
    }
    if (benefit && category) {
        const createClaimOptions = await claimParamsToCreateClaimOptions({ ...opts, benefit, category }, accessToken);
        await createClaim(createClaimOptions);
    }
    else if (openaiApiKey) {
        const benefitsWithCategories = await getBenefitsWithCategories(accessToken);
        const { benefit, category } = await attemptToinferCategoryAndBenefit({
            merchant: opts.merchant,
            description: opts.description,
            benefitsWithCategories,
            openaiApiKey,
        });
        const createClaimOptions = await claimParamsToCreateClaimOptions({ ...opts, benefit, category }, accessToken);
        await createClaim(createClaimOptions);
    }
    else {
        throw new Error('You must either specify --benefit and --category, or an OpenAI API key.');
    }
    console.log(chalk.green('Claim submitted successfully ✅'));
}));
export default command;
