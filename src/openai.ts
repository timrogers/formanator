import { Configuration, OpenAIApi } from 'openai';
import chalk from 'chalk';

import { prompt } from './utils.js';
import { type BenefitWithCategories } from './forma.js';

export const attemptToInferCategoryAndBenefit = async (opts: {
  merchant: string;
  description: string;
  benefitsWithCategories: BenefitWithCategories[];
  openaiApiKey: string;
}): Promise<{ category: string; benefit: string }> => {
  const { merchant, description, benefitsWithCategories, openaiApiKey: apiKey } = opts;

  const configuration = new Configuration({
    apiKey,
  });

  const openai = new OpenAIApi(configuration);

  const categoriesWithBenefits = benefitsWithCategories.flatMap((benefit) =>
    benefit.categories.map((category) => ({ ...category, benefit })),
  );

  const validCategories = categoriesWithBenefits.flatMap(
    (category) => category.subcategory_alias ?? category.subcategory_name,
  );

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
    throw new Error(
      `Something went wrong while inferring the benefit and category for your claim. OpenAI returned an unexpected response: ${JSON.stringify(
        chatCompletion.data,
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
    `OpenAI inferred that you should claim using the ${chalk.magenta(
      returnedCategory.benefit.name,
    )} benefit and ${chalk.magenta(
      returnedCategoryAsString,
    )} category. If that seems right, hit Enter. If not, press Ctrl + C to end your session.`,
  );
  prompt('> ');

  return { category: returnedCategoryAsString, benefit: returnedCategory.benefit.name };
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
