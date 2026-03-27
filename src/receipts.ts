import { readFileSync, readdirSync, existsSync, statSync } from 'fs';
import { extname, join } from 'path';

// Supported file extensions for receipts
export const SUPPORTED_EXTENSIONS = ['.jpg', '.jpeg', '.png', '.pdf', '.heic'];

// Check if a file is a supported receipt type
export const isSupportedReceiptFile = (filename: string): boolean => {
  const ext = extname(filename).toLowerCase();
  return SUPPORTED_EXTENSIONS.includes(ext);
};

// Get MIME type from file extension
export const getMimeType = (filePath: string): string => {
  const ext = extname(filePath).toLowerCase();
  switch (ext) {
    case '.jpg':
    case '.jpeg':
      return 'image/jpeg';
    case '.png':
      return 'image/png';
    case '.heic':
      return 'image/heic';
    case '.pdf':
      return 'application/pdf';
    default:
      return 'application/octet-stream';
  }
};

// Convert PDF to JPEG image if needed, returns original path for non-PDFs
export const convertToImageIfNeeded = async (filePath: string): Promise<string> => {
  const fileExtension = extname(filePath).toLowerCase();

  if (fileExtension === '.pdf') {
    try {
      // Dynamic import to handle CommonJS module
      const pdf2pic = await import('pdf2pic');

      const convertOptions = {
        density: 100,
        saveFilename: 'page',
        savePath: '/tmp',
        format: 'jpeg',
        width: 2000,
        height: 2000,
      };

      const convert = pdf2pic.fromPath(filePath, convertOptions);
      const result = await convert(1, { responseType: 'image' });

      return result.path as string;
    } catch (error) {
      throw new Error(
        `Failed to convert PDF to image. Please ensure GraphicsMagick and Ghostscript are installed on your system, or use JPEG/PNG receipts instead. Error: ${error}`,
      );
    }
  }

  return filePath;
};

// Encode an image file to base64
export const encodeImageToBase64 = (imagePath: string): string => {
  const imageBuffer = readFileSync(imagePath);
  return imageBuffer.toString('base64');
};

// Get all receipt files from a directory
export const getReceiptFiles = (directory: string): string[] => {
  if (!existsSync(directory)) {
    throw new Error(`Directory '${directory}' does not exist.`);
  }

  try {
    const files = readdirSync(directory);
    return files.filter(isSupportedReceiptFile).map((file) => join(directory, file));
  } catch (error) {
    throw new Error(
      `Could not read directory '${directory}': ${
        error instanceof Error ? error.message : String(error)
      }`,
    );
  }
};

export interface ReceiptFileInfo {
  name: string;
  path: string;
  extension: string;
  sizeBytes: number;
}

// Get detailed info about receipt files in a directory
export const getReceiptFileInfos = (directory: string): ReceiptFileInfo[] => {
  const files = getReceiptFiles(directory);
  return files.map((filePath) => {
    const stats = statSync(filePath);
    return {
      name: filePath.split('/').pop() || filePath,
      path: filePath,
      extension: extname(filePath).toLowerCase(),
      sizeBytes: stats.size,
    };
  });
};
