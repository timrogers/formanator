/**
 * Shared helper functions for API URL construction and query parameter handling.
 * This module centralizes the API base URL and standardizes URL building patterns.
 */

export const API_BASE_URL = 'https://api.joinforma.com';

/**
 * Build a complete API URL from a path and optional query parameters.
 *
 * @param path - The API path (e.g., '/client/api/v3/settings/profile')
 * @param queryParams - Optional query parameters as key-value pairs
 * @returns Complete URL string
 */
export const buildApiUrl = (
  path: string,
  queryParams?: Record<string, string>,
): string => {
  const url = new URL(path, API_BASE_URL);

  if (queryParams) {
    const searchParams = new URLSearchParams(queryParams);
    url.search = searchParams.toString();
  }

  return url.toString();
};

/**
 * Common query parameters used across API calls
 */
export const COMMON_QUERY_PARAMS = {
  IS_MOBILE: 'true',
} as const;
