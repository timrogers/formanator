import OpenAI from 'openai';
import chalk from 'chalk';
import { readFileSync } from 'fs';
import { extname } from 'path';

import { prompt } from './utils.js';
import { type BenefitWithCategories } from './forma.js';

const prepareOpenAIClient = (
  apiKey?: string,
  githubToken?: string,
): { openai: OpenAI; model: string } => {
  if (apiKey && githubToken) {
    console.warn(
      chalk.yellow(
        'Warning: You have provided both an OpenAI API Key and a GitHub Token. Defaulting to using OpenAI.',
      ),
    );
  }
  if (apiKey) {
    return {
      openai: new OpenAI({
        apiKey,
      }),
      model: 'gpt-4o',
    };
  } else if (githubToken) {
    return {
      openai: new OpenAI({
        baseURL: 'https://models.github.ai/inference',
        apiKey: githubToken,
      }),
      model: 'openai/gpt-4.1',
    };
  } else {
    throw new Error('You must either specify a GitHub Token or OpenAI API Key');
  }
};

export const attemptToInferCategoryAndBenefit = async (opts: {
  merchant: string;
  description: string;
  benefitsWithCategories: BenefitWithCategories[];
  openaiApiKey?: string;
  githubToken?: string;
}): Promise<{ category: string; benefit: string }> => {
  const { merchant, description, benefitsWithCategories, openaiApiKey, githubToken } =
    opts;

  const { openai, model } = prepareOpenAIClient(openaiApiKey, githubToken);

  const categoriesWithBenefits = benefitsWithCategories.flatMap((benefit) =>
    benefit.categories.map((category) => ({ ...category, benefit })),
  );

  const validCategories = categoriesWithBenefits.flatMap(
    (category) => category.subcategory_alias ?? category.subcategory_name,
  );

  const content = generateOpenaiPrompt({ validCategories, merchant, description });

  const chatCompletion = await openai.chat.completions.create({
    model,
    messages: [
      {
        role: 'user',
        content,
      },
    ],
  });

  const returnedCategoryAsString = chatCompletion.choices[0].message?.content;

  if (!returnedCategoryAsString) {
    throw new Error(
      `Something went wrong while inferring the benefit and category for your claim. OpenAI returned an unexpected response: ${JSON.stringify(
        chatCompletion,
      )}`,
    );
  }

  const returnedCategory = categoriesWithBenefits.find(
    (category) =>
      category.subcategory_alias === returnedCategoryAsString ||
      category.subcategory_name === returnedCategoryAsString,
  );

  if (!returnedCategory) {
    throw new Error(
      `Something went wrong while inferring the benefit and category for your claim. OpenAI returned a response that wasn't a valid category: ${returnedCategoryAsString}`,
    );
  }

  console.log(
    `${openaiApiKey ? 'OpenAI' : 'GitHub Models'} inferred that you should claim using the ${chalk.magenta(
      returnedCategory.benefit.name,
    )} benefit and ${chalk.magenta(
      returnedCategoryAsString,
    )} category. If that seems right, hit Enter. If not, press Ctrl + C to end your session.`,
  );
  prompt('> ');

  return { category: returnedCategoryAsString, benefit: returnedCategory.benefit.name };
};

export interface ReceiptInferenceResult {
  amount: string;
  merchant: string;
  purchaseDate: string;
  description: string;
  category: string;
  benefit: string;
}

export const attemptToInferAllFromReceipt = async (opts: {
  receiptPath: string;
  benefitsWithCategories: BenefitWithCategories[];
  openaiApiKey?: string;
  githubToken?: string;
}): Promise<ReceiptInferenceResult> => {
  const { receiptPath, benefitsWithCategories, openaiApiKey, githubToken } = opts;

  const { openai, model } = prepareOpenAIClient(openaiApiKey, githubToken);

  // Convert PDF to image if needed
  const imagePath = await convertToImageIfNeeded(receiptPath);

  // Encode image to base64
  const imageBase64 = encodeImageToBase64(imagePath);

  const categoriesWithBenefits = benefitsWithCategories.flatMap((benefit) =>
    benefit.categories.map((category) => ({ ...category, benefit })),
  );

  const validCategories = categoriesWithBenefits.flatMap(
    (category) => category.subcategory_alias ?? category.subcategory_name,
  );

  const validBenefits = benefitsWithCategories.map((benefit) => benefit.name);

  const content = generateReceiptInferencePrompt({ validCategories, validBenefits });

  const chatCompletion = await openai.chat.completions.create({
    model,
    messages: [
      {
        role: 'user',
        content: [
          {
            type: 'text',
            text: content,
          },
          {
            type: 'image_url',
            image_url: {
              url: `data:image/jpeg;base64,${imageBase64}`,
            },
          },
        ],
      },
    ],
  });

  const returnedResponseAsString = chatCompletion.choices[0].message?.content;

  if (!returnedResponseAsString) {
    throw new Error(
      `Something went wrong while inferring claim details from your receipt. OpenAI returned an unexpected response: ${JSON.stringify(
        chatCompletion,
      )}`,
    );
  }

  // Parse the JSON response
  let parsedResponse: ReceiptInferenceResult;
  try {
    parsedResponse = JSON.parse(returnedResponseAsString);
  } catch (error) {
    throw new Error(
      `Something went wrong while parsing OpenAI's response: ${returnedResponseAsString}. Error: ${error}`,
    );
  }

  // Validate the benefit exists
  const matchingBenefit = benefitsWithCategories.find(
    (benefit) => benefit.name === parsedResponse.benefit,
  );

  if (!matchingBenefit) {
    throw new Error(
      `Something went wrong while inferring the claim details. OpenAI returned a benefit that wasn't valid: ${parsedResponse.benefit}`,
    );
  }

  // Validate the category exists for the benefit
  const returnedCategory = categoriesWithBenefits.find(
    (category) =>
      category.benefit.name === parsedResponse.benefit &&
      (category.subcategory_alias === parsedResponse.category ||
        category.subcategory_name === parsedResponse.category),
  );

  if (!returnedCategory) {
    throw new Error(
      `Something went wrong while inferring the claim details. OpenAI returned a category that wasn't valid for the benefit: ${parsedResponse.category}`,
    );
  }

  console.log(chalk.cyan('OpenAI inferred the following details from your receipt:'));
  console.log(`Amount: ${chalk.magenta(parsedResponse.amount)}`);
  console.log(`Merchant: ${chalk.magenta(parsedResponse.merchant)}`);
  console.log(`Purchase Date: ${chalk.magenta(parsedResponse.purchaseDate)}`);
  console.log(`Description: ${chalk.magenta(parsedResponse.description)}`);
  console.log(`Benefit: ${chalk.magenta(parsedResponse.benefit)}`);
  console.log(`Category: ${chalk.magenta(parsedResponse.category)}`);
  console.log();
  console.log(
    `If these details look correct, hit Enter to proceed. If not, press Ctrl + C to end your session.`,
  );
  prompt('> ');

  return parsedResponse;
};

const generateOpenaiPrompt = (opts: {
  validCategories: string[];
  merchant: string;
  description: string;
}): string => {
  const { description, merchant, validCategories } = opts;

  return `Your job is to predict the category for an expense claim based on the name of the merchant and a description of what was purchased. You should give a single, specific answer without any extra words or punctuation.

Here are the possible categories:

${validCategories.join('\n')}

Please predict the category for the following claim:

Merchant: ${merchant}
Description: ${description}`;
};

const generateReceiptInferencePrompt = (opts: {
  validCategories: string[];
  validBenefits: string[];
}): string => {
  const { validCategories, validBenefits } = opts;

  return `Your job is to analyze a receipt image and extract ALL required information for an expense claim. You must return a JSON object with the following fields:

- amount: The total amount (e.g., "25.99")
- merchant: The name of the merchant/store
- purchaseDate: The date in YYYY-MM-DD format
- description: A brief description of what was purchased
- benefit: The most appropriate benefit category from the valid benefits list
- category: The most appropriate category from the valid categories list

Valid benefits:
${validBenefits.join('\n')}

Valid categories:
${validCategories.join('\n')}

Return ONLY a valid JSON object with these exact field names. Do not include any other text or formatting.`;
};

const convertToImageIfNeeded = async (filePath: string): Promise<string> => {
  const fileExtension = extname(filePath).toLowerCase();

  if (fileExtension === '.pdf') {
    // Import pdf2pic dynamically to handle potential import issues
    const { fromPath } = await import('pdf2pic');

    const convertOptions = {
      density: 100, // output pixels per inch
      saveFilename: 'page', // output file name
      savePath: '/tmp', // output directory
      format: 'jpeg', // output format
      width: 2000, // output width
      height: 2000, // output height
    };

    const convert = fromPath(filePath, convertOptions);
    const result = await convert(1, { responseType: 'image' }); // Convert first page only

    return result.path as string;
  }

  // For non-PDF files (JPEG, PNG, HEIC), return the original path
  return filePath;
};

const encodeImageToBase64 = (imagePath: string): string => {
  const imageBuffer = readFileSync(imagePath);
  return imageBuffer.toString('base64');
};
