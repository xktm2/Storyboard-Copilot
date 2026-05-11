import type { ImageModelDefinition } from '../../types';
import { createFixedResolutionPricing } from '@/features/canvas/pricing';

export const APIYI_SEDREAM_5_MODEL_ID = 'apiyi/seedream-5';

const SEEDREAM_5_ASPECT_RATIOS = [
  '1:1',
  '4:3',
  '3:4',
  '16:9',
  '9:16',
  '3:2',
  '2:3',
  '21:9',
] as const;

export const imageModel: ImageModelDefinition = {
  id: APIYI_SEDREAM_5_MODEL_ID,
  mediaType: 'image',
  displayName: 'Seedream 5.0',
  providerId: 'apiyi',
  description: 'apiyi · Seedream 5.0 字节方舟图像生成与编辑，支持多图融合',
  eta: '2min',
  expectedDurationMs: 120000,
  defaultAspectRatio: '1:1',
  defaultResolution: '2K',
  aspectRatios: SEEDREAM_5_ASPECT_RATIOS.map((value) => ({ value, label: value })),
  resolutions: [
    { value: '2K', label: '2K' },
    { value: '3K', label: '3K' },
  ],
  pricing: createFixedResolutionPricing({
    currency: 'USD',
    standardRates: {
      '2K': 0.04,
      '3K': 0.06,
    },
  }),
  resolveRequest: ({ referenceImageCount }) => ({
    requestModel: APIYI_SEDREAM_5_MODEL_ID,
    modeLabel: referenceImageCount > 0 ? '编辑模式' : '生成模式',
  }),
};
