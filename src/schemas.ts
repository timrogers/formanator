import { z } from 'zod';

// Profile Response Schema
const ProfileResponseCategorySchema = z.object({
  id: z.string(),
  name: z.string(),
  subcategories: z.array(
    z.object({
      name: z.string(),
      value: z.string(),
      aliases: z.array(z.string()),
    }),
  ),
});

export const ProfileResponseSchema = z.object({
  data: z.object({
    company: z.object({
      company_wallet_configurations: z.array(
        z.object({
          id: z.string(),
          wallet_name: z.string(),
          categories: z.array(ProfileResponseCategorySchema),
        }),
      ),
    }),
    employee: z.object({
      employee_wallets: z.array(
        z.object({
          id: z.string(),
          amount: z.number(),
          company_wallet_configuration: z.object({
            wallet_name: z.string(),
          }),
          is_employee_eligible: z.boolean(),
        }),
      ),
      settings: z.object({
        currency: z.string(),
      }),
    }),
  }),
});

// Claims List Response Schema
export const ClaimsListResponseSchema = z.object({
  data: z.object({
    claims: z.array(
      z.object({
        id: z.string(),
        status: z.string(),
        reimbursement: z.object({
          status: z.string(),
          payout_status: z.string(),
          amount: z.number(),
          category: z.string(),
          subcategory: z.string(),
          reimbursement_vendor: z.string(),
          date_processed: z.string(),
          note: z.string(),
          employee_note: z.string(),
        }),
      }),
    ),
  }),
});

// Create Claim Response Schema
export const CreateClaimResponseSchema = z.object({
  success: z.boolean(),
});

// Request Magic Link Response Schema
export const RequestMagicLinkResponseSchema = z.object({
  success: z.boolean(),
  status: z.number(),
  data: z.object({
    done: z.boolean(),
  }),
});

// Exchange ID and TK for Access Token Response Schema
export const ExchangeIdAndTkForAccessTokenResponseSchema = z.object({
  success: z.boolean(),
  status: z.number(),
  data: z.object({
    auth_token: z.string(),
  }),
});

// Type inference from schemas
export type ProfileResponse = z.infer<typeof ProfileResponseSchema>;
export type ClaimsListResponse = z.infer<typeof ClaimsListResponseSchema>;
export type CreateClaimResponse = z.infer<typeof CreateClaimResponseSchema>;
export type RequestMagicLinkResponse = z.infer<typeof RequestMagicLinkResponseSchema>;
export type ExchangeIdAndTkForAccessTokenResponse = z.infer<
  typeof ExchangeIdAndTkForAccessTokenResponseSchema
>;
