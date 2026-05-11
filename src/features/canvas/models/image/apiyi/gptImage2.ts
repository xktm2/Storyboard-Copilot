import type { ImageModelDefinition } from '../../types';
import { createFixedResolutionPricing } from '@/features/canvas/pricing';

export const APIYI_GPT_IMAGE_2_MODEL_ID = 'apiyi/gpt-image-2';

const GPT_IMAGE_2_ASPECT_RATIOS = [
  '1:1',
  '4:3',
  '3:4',
  '3:2',
  '2:3',
  '16:9',
  '9:16',
  '21:9',
] as const;

export const imageModel: ImageModelDefinition = {
  id: APIYI_GPT_IMAGE_2_MODEL_ID,
  mediaType: 'image',
  displayName: 'GPT Image 2',
  providerId: 'apiyi',
  description: 'apiyi · GPT Image 2 图像生成与编辑',
  eta: '2min',
  expectedDurationMs: 120000,
  defaultAspectRatio: '1:1',
  defaultResolution: '1K',
  aspectRatios: GPT_IMAGE_2_ASPECT_RATIOS.map((value) => ({ value, label: value })),
  resolutions: [
    { value: '1K', label: '1K' },
    { value: '2K', label: '2K' },
    { value: '4K', label: '4K' },
  ],
  pricing: createFixedResolutionPricing({
    currency: 'USD',
    standardRates: {
      '1K': 0.04,
      '2K': 0.06,
      '4K': 0.09,
    },
  }),
  resolveRequest: ({ referenceImageCount }) => ({
    requestModel: APIYI_GPT_IMAGE_2_MODEL_ID,
    modeLabel: referenceImageCount > 0 ? '编辑模式' : '生成模式',
  }),
};
