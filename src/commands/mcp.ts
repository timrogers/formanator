import * as commander from 'commander';
import { Server } from '@modelcontextprotocol/sdk/server/index.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import {
  CallToolRequestSchema,
  ErrorCode,
  ListToolsRequestSchema,
  McpError,
} from '@modelcontextprotocol/sdk/types.js';

import { actionRunner } from '../utils.js';
import { getAccessToken, getIntegrationAuth, storeIntegrationAuth } from '../config.js';
import { getBenefitsWithCategories, getClaimsList, createClaim } from '../forma.js';
import { claimParamsToCreateClaimOptions } from '../claims.js';
import {
  convertToImageIfNeeded,
  encodeImageToBase64,
  getMimeType,
  getReceiptFileInfos,
  SUPPORTED_EXTENSIONS,
} from '../receipts.js';
import { listProviders, getProvider } from '../integrations/registry.js';
import VERSION from '../version.js';

const command = new commander.Command();

interface Arguments {
  accessToken?: string;
}

// MCP Server implementation
const createMcpServer = () => {
  const server = new Server(
    {
      name: 'formanator',
      version: VERSION,
    },
    {
      capabilities: {
        tools: {},
      },
    },
  );

  // List available tools
  server.setRequestHandler(ListToolsRequestSchema, async () => {
    return {
      tools: [
        {
          name: 'listBenefitsWithCategories',
          description:
            'List all available Forma benefits with their categories and remaining balances',
          inputSchema: {
            type: 'object',
            properties: {},
          },
        },
        {
          name: 'listClaims',
          description: 'List claims in your Forma account with optional filtering',
          inputSchema: {
            type: 'object',
            properties: {
              filter: {
                type: 'string',
                description:
                  'Filter claims by status or reimbursement status (currently supports: in_progress)',
                enum: ['in_progress'],
              },
            },
          },
        },
        {
          name: 'createClaim',
          description: 'Create a new Forma claim',
          inputSchema: {
            type: 'object',
            properties: {
              amount: {
                type: 'string',
                description: 'The amount to claim (e.g., "25.99")',
              },
              merchant: {
                type: 'string',
                description: 'The merchant/vendor name',
              },
              purchaseDate: {
                type: 'string',
                description: 'Purchase date in YYYY-MM-DD format',
              },
              description: {
                type: 'string',
                description: 'Description of the purchase',
              },
              receiptPath: {
                type: 'array',
                items: {
                  type: 'string',
                },
                description: 'Array of file paths to receipt images/PDFs',
              },
              benefit: {
                type: 'string',
                description: 'The benefit name to claim against',
              },
              category: {
                type: 'string',
                description: 'The category name or alias for the claim',
              },
            },
            required: [
              'amount',
              'merchant',
              'purchaseDate',
              'description',
              'receiptPath',
              'benefit',
              'category',
            ],
          },
        },
        {
          name: 'readReceipt',
          description:
            'Read a receipt image file and return it for visual analysis along with available Forma benefits and categories. Use this to analyze receipts and extract claim details (amount, merchant, date, description, benefit, category).',
          inputSchema: {
            type: 'object',
            properties: {
              filePath: {
                type: 'string',
                description:
                  'Absolute path to a receipt file (JPEG, PNG, HEIC, or PDF)',
              },
            },
            required: ['filePath'],
          },
        },
        {
          name: 'scanReceiptDirectory',
          description:
            'List all supported receipt files in a directory. Returns file names, paths, extensions, and sizes.',
          inputSchema: {
            type: 'object',
            properties: {
              directoryPath: {
                type: 'string',
                description: 'Absolute path to a directory containing receipt files',
              },
            },
            required: ['directoryPath'],
          },
        },
        {
          name: 'listIntegrations',
          description:
            'List available receipt-fetching integrations (e.g., Uber, Bell, Amex) and whether they are configured with authentication.',
          inputSchema: {
            type: 'object',
            properties: {},
          },
        },
        {
          name: 'configureIntegration',
          description:
            'Configure authentication cookies for a receipt-fetching integration. Cookies are stored locally and validated against the provider.',
          inputSchema: {
            type: 'object',
            properties: {
              provider: {
                type: 'string',
                description: 'Integration provider name (e.g., "uber")',
              },
              cookies: {
                type: 'object',
                description:
                  'Cookie key-value pairs for authentication (e.g., {"sid": "...", "csid": "..."})',
              },
            },
            required: ['provider', 'cookies'],
          },
        },
        {
          name: 'fetchReceipts',
          description:
            'List available receipts from an integration provider. Returns receipt metadata including dates, amounts, and descriptions.',
          inputSchema: {
            type: 'object',
            properties: {
              provider: {
                type: 'string',
                description: 'Integration provider name (e.g., "uber")',
              },
              startDate: {
                type: 'string',
                description: 'Start date filter in YYYY-MM-DD format',
              },
              endDate: {
                type: 'string',
                description: 'End date filter in YYYY-MM-DD format',
              },
            },
            required: ['provider'],
          },
        },
        {
          name: 'downloadReceipt',
          description:
            'Download a receipt from an integration provider and return it as an image for visual analysis, along with available Forma benefits and categories.',
          inputSchema: {
            type: 'object',
            properties: {
              provider: {
                type: 'string',
                description: 'Integration provider name (e.g., "uber")',
              },
              receiptId: {
                type: 'string',
                description: 'Receipt ID from the fetchReceipts results',
              },
            },
            required: ['provider', 'receiptId'],
          },
        },
      ],
    };
  });

  // Handle tool calls
  server.setRequestHandler(CallToolRequestSchema, async (request) => {
    const { name, arguments: args } = request.params;

    // Fetch access token for each tool invocation
    const accessToken = getAccessToken();
    if (!accessToken) {
      throw new McpError(
        ErrorCode.InternalError,
        "You aren't logged in to Forma. Please run `npx formanator login` first.",
      );
    }

    try {
      switch (name) {
        case 'listBenefitsWithCategories': {
          const benefits = await getBenefitsWithCategories(accessToken);
          return {
            content: [
              {
                type: 'text',
                text: JSON.stringify(benefits, null, 2),
              },
            ],
          };
        }

        case 'listClaims': {
          const { filter } = args as { filter?: 'in_progress' };
          const claims = await getClaimsList(accessToken, filter);
          return {
            content: [
              {
                type: 'text',
                text: JSON.stringify(claims, null, 2),
              },
            ],
          };
        }

        case 'createClaim': {
          const {
            amount,
            merchant,
            purchaseDate,
            description,
            receiptPath,
            benefit,
            category,
          } = args as {
            amount: string;
            merchant: string;
            purchaseDate: string;
            description: string;
            receiptPath: string[];
            benefit: string;
            category: string;
          };

          // Convert claim parameters to create claim options
          const createClaimOptions = await claimParamsToCreateClaimOptions(
            {
              amount,
              merchant,
              purchaseDate,
              description,
              receiptPath,
              benefit,
              category,
            },
            accessToken,
          );

          await createClaim(createClaimOptions);

          return {
            content: [
              {
                type: 'text',
                text: 'Claim created successfully',
              },
            ],
          };
        }

        case 'readReceipt': {
          const { filePath } = args as { filePath: string };

          // Convert PDF to image if needed, then encode
          const imagePath = await convertToImageIfNeeded(filePath);
          const imageBase64 = encodeImageToBase64(imagePath);
          const mimeType = getMimeType(imagePath);

          // Fetch benefits context
          const benefits = await getBenefitsWithCategories(accessToken);

          const validBenefits = benefits.map((b) => b.name);
          const validCategories = benefits.flatMap((b) =>
            b.categories.map(
              (c) => `${b.name} → ${c.subcategory_alias ?? c.subcategory_name}`,
            ),
          );

          const contextText = `Please analyze this receipt image and extract the following claim details:
- amount: The total amount (e.g., "25.99")
- merchant: The name of the merchant/store
- purchaseDate: The date in YYYY-MM-DD format
- description: A brief description of what was purchased
- benefit: The most appropriate benefit from the list below
- category: The most appropriate category from the list below

Available benefits: ${validBenefits.join(', ')}

Available categories (benefit → category):
${validCategories.join('\n')}

After analyzing, you can submit the claim using the createClaim tool.`;

          return {
            content: [
              {
                type: 'image' as const,
                data: imageBase64,
                mimeType,
              },
              {
                type: 'text' as const,
                text: contextText,
              },
            ],
          };
        }

        case 'scanReceiptDirectory': {
          const { directoryPath } = args as { directoryPath: string };
          const fileInfos = getReceiptFileInfos(directoryPath);

          return {
            content: [
              {
                type: 'text',
                text: JSON.stringify(
                  {
                    directory: directoryPath,
                    files: fileInfos,
                    supportedExtensions: SUPPORTED_EXTENSIONS,
                    count: fileInfos.length,
                  },
                  null,
                  2,
                ),
              },
            ],
          };
        }

        case 'listIntegrations': {
          const providers = listProviders();
          const integrationList = providers.map((p) => {
            const auth = getIntegrationAuth(p.name);
            return {
              name: p.name,
              displayName: p.displayName,
              configured: !!auth,
            };
          });

          return {
            content: [
              {
                type: 'text',
                text: JSON.stringify(integrationList, null, 2),
              },
            ],
          };
        }

        case 'configureIntegration': {
          const { provider: providerName, cookies } = args as {
            provider: string;
            cookies: Record<string, string>;
          };

          const provider = getProvider(providerName);
          if (!provider) {
            throw new Error(
              `Unknown integration provider: ${providerName}. Use listIntegrations to see available providers.`,
            );
          }

          const isValid = await provider.validateAuth({ cookies });
          storeIntegrationAuth(providerName, cookies);

          return {
            content: [
              {
                type: 'text',
                text: isValid
                  ? `Successfully configured ${provider.displayName} integration. Cookies are valid.`
                  : `Stored cookies for ${provider.displayName}, but validation failed. The cookies may be expired — try refreshing them.`,
              },
            ],
          };
        }

        case 'fetchReceipts': {
          const {
            provider: providerName,
            startDate,
            endDate,
          } = args as {
            provider: string;
            startDate?: string;
            endDate?: string;
          };

          const provider = getProvider(providerName);
          if (!provider) {
            throw new Error(
              `Unknown integration provider: ${providerName}. Use listIntegrations to see available providers.`,
            );
          }

          const auth = getIntegrationAuth(providerName);
          if (!auth) {
            throw new Error(
              `Integration ${providerName} is not configured. Use configureIntegration first.`,
            );
          }

          const receipts = await provider.listReceipts(
            { cookies: auth.cookies },
            { startDate, endDate },
          );

          return {
            content: [
              {
                type: 'text',
                text: JSON.stringify(receipts, null, 2),
              },
            ],
          };
        }

        case 'downloadReceipt': {
          const { provider: providerName, receiptId } = args as {
            provider: string;
            receiptId: string;
          };

          const provider = getProvider(providerName);
          if (!provider) {
            throw new Error(
              `Unknown integration provider: ${providerName}. Use listIntegrations to see available providers.`,
            );
          }

          const auth = getIntegrationAuth(providerName);
          if (!auth) {
            throw new Error(
              `Integration ${providerName} is not configured. Use configureIntegration first.`,
            );
          }

          const receipt = await provider.downloadReceipt(
            { cookies: auth.cookies },
            receiptId,
          );

          // If the downloaded file is a PDF, we get image/jpeg after conversion
          // For images, use the original mime type
          let imageBase64: string;
          let mimeType: string;

          if (receipt.mimeType === 'application/pdf') {
            // Write PDF to temp file, convert, then encode
            const { writeFileSync } = await import('fs');
            const tmpPath = `/tmp/receipt-${receiptId}.pdf`;
            writeFileSync(tmpPath, receipt.data);
            const imagePath = await convertToImageIfNeeded(tmpPath);
            imageBase64 = encodeImageToBase64(imagePath);
            mimeType = 'image/jpeg';
          } else {
            imageBase64 = receipt.data.toString('base64');
            mimeType = receipt.mimeType;
          }

          // Fetch benefits context
          const benefits = await getBenefitsWithCategories(accessToken);
          const validBenefits = benefits.map((b) => b.name);
          const validCategories = benefits.flatMap((b) =>
            b.categories.map(
              (c) => `${b.name} → ${c.subcategory_alias ?? c.subcategory_name}`,
            ),
          );

          const contextText = `Please analyze this receipt image and extract the following claim details:
- amount: The total amount (e.g., "25.99")
- merchant: The name of the merchant/store
- purchaseDate: The date in YYYY-MM-DD format
- description: A brief description of what was purchased
- benefit: The most appropriate benefit from the list below
- category: The most appropriate category from the list below

Available benefits: ${validBenefits.join(', ')}

Available categories (benefit → category):
${validCategories.join('\n')}

After analyzing, you can submit the claim using the createClaim tool.`;

          return {
            content: [
              {
                type: 'image' as const,
                data: imageBase64,
                mimeType,
              },
              {
                type: 'text' as const,
                text: contextText,
              },
            ],
          };
        }

        default:
          throw new McpError(ErrorCode.MethodNotFound, `Unknown tool: ${name}`);
      }
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error);
      throw new McpError(
        ErrorCode.InternalError,
        `Error executing tool ${name}: ${errorMessage}`,
      );
    }
  });

  return server;
};

command
  .name('mcp')
  .version(VERSION)
  .description('Run Formanator as an MCP (Model Context Protocol) server')
  .option('--access-token <access_token>', 'Access token used to authenticate with Forma')
  .action(
    actionRunner(async (opts: Arguments) => {
      const accessToken = opts.accessToken ?? getAccessToken();

      if (!accessToken) {
        throw new Error(
          "You aren't logged in to Forma. Please run `npx formanator login` first.",
        );
      }

      const server = createMcpServer();
      const transport = new StdioServerTransport();

      await server.connect(transport);

      // Keep the server running
      console.error('Formanator MCP server started');
    }),
  );

export default command;
