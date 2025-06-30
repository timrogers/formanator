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
import { getAccessToken } from '../config.js';
import { getBenefitsWithCategories, getClaimsList, createClaim } from '../forma.js';
import { claimParamsToCreateClaimOptions } from '../claims.js';
import VERSION from '../version.js';

const command = new commander.Command();

interface Arguments {
  accessToken?: string;
}

// MCP Server implementation
const createMcpServer = (accessToken: string) => {
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
          description: 'List claims in your Forma account with pagination support',
          inputSchema: {
            type: 'object',
            properties: {
              page: {
                type: 'number',
                description: 'Page number to retrieve (default: 0)',
                default: 0,
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
      ],
    };
  });

  // Handle tool calls
  server.setRequestHandler(CallToolRequestSchema, async (request) => {
    const { name, arguments: args } = request.params;

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
          const page = (args as { page?: number })?.page || 0;
          const claims = await getClaimsList(accessToken, page);
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
          "You aren't logged in to Forma. Please run `formanator login` first.",
        );
      }

      const server = createMcpServer(accessToken);
      const transport = new StdioServerTransport();

      await server.connect(transport);

      // Keep the server running
      console.error('Formanator MCP server started');
    }),
  );

export default command;
