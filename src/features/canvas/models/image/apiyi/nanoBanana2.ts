import type { ImageModelDefinition } from '../../types';
import { createFixedResolutionPricing } from '@/features/canvas/pricing';

export const APIYI_NANO_BANANA_2_MODEL_ID = 'apiyi/nano-banana-2';

const NANO_BANANA_2_ASPECT_RATIOS = [
  '1:1',
  '1:4',
  '1:8',
  '9:16',
  '16:9',
  '3:4',
  '4:3',
  '4:1',
  '8:1',
  '2:3',
  '3:2',
  '5:4',
  '4:5',
  '21:9',
] as const;

export const imageModel: ImageModelDefinition = {
  id: APIYI_NANO_BANANA_2_MODEL_ID,
  mediaType: 'image',
  displayName: 'Nano Banana 2',
  providerId: 'apiyi',
  description: 'apiyi · Nano Banana 2 图像生成与编辑，支持深度推理',
  eta: '1min',
  expectedDurationMs: 60000,
  defaultAspectRatio: '1:1',
  defaultResolution: '1K',
  aspectRatios: NANO_BANANA_2_ASPECT_RATIOS.map((value) => ({ value, label: value })),
  resolutions: [
    { value: '512', label: '512px' },
    { value: '1K', label: '1K' },
    { value: '2K', label: '2K' },
    { value: '4K', label: '4K' },
  ],
  pricing: createFixedResolutionPricing({
    currency: 'USD',
    standardRates: {
      '512': 0.01,
      '1K': 0.025,
      '2K': 0.04,
      '4K': 0.06,
    },
  }),
  resolveRequest: ({ referenceImageCount }) => ({
    requestModel: APIYI_NANO_BANANA_2_MODEL_ID,
    modeLabel: referenceImageCount > 0 ? '编辑模式' : '生成模式',
  }),
};
