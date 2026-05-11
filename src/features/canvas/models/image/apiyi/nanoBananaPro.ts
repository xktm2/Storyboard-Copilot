import type { ImageModelDefinition } from '../../types';
import { createFixedResolutionPricing } from '@/features/canvas/pricing';

export const APIYI_NANO_BANANA_PRO_MODEL_ID = 'apiyi/nano-banana-pro';

const NANO_BANANA_PRO_ASPECT_RATIOS = [
  '1:1',
  '4:3',
  '3:4',
  '3:2',
  '2:3',
  '16:9',
  '9:16',
  '4:5',
  '5:4',
  '21:9',
] as const;

export const imageModel: ImageModelDefinition = {
  id: APIYI_NANO_BANANA_PRO_MODEL_ID,
  mediaType: 'image',
  displayName: 'Nano Banana Pro',
  providerId: 'apiyi',
  description: 'apiyi · Nano Banana Pro 极致画质图像生成与编辑',
  eta: '1min',
  expectedDurationMs: 60000,
  defaultAspectRatio: '1:1',
  defaultResolution: '1K',
  aspectRatios: NANO_BANANA_PRO_ASPECT_RATIOS.map((value) => ({ value, label: value })),
  resolutions: [
    { value: '1K', label: '1K' },
    { value: '2K', label: '2K' },
    { value: '4K', label: '4K' },
  ],
  pricing: createFixedResolutionPricing({
    currency: 'USD',
    standardRates: {
      '1K': 0.09,
      '2K': 0.09,
      '4K': 0.12,
    },
  }),
  resolveRequest: ({ referenceImageCount }) => ({
    requestModel: APIYI_NANO_BANANA_PRO_MODEL_ID,
    modeLabel: referenceImageCount > 0 ? '编辑模式' : '生成模式',
  }),
};
