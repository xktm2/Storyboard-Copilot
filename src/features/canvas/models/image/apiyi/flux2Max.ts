import type { ImageModelDefinition } from '../../types';
import { createFixedResolutionPricing } from '@/features/canvas/pricing';

export const APIYI_FLUX2_MAX_MODEL_ID = 'apiyi/flux-2-max';

const FLUX2_MAX_ASPECT_RATIOS = [
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
  id: APIYI_FLUX2_MAX_MODEL_ID,
  mediaType: 'image',
  displayName: 'FLUX.2 Max',
  providerId: 'apiyi',
  description: 'apiyi · FLUX.2 Max 旗舰画质图像生成与编辑',
  eta: '3min',
  expectedDurationMs: 180000,
  defaultAspectRatio: '1:1',
  defaultResolution: '2MP',
  aspectRatios: FLUX2_MAX_ASPECT_RATIOS.map((value) => ({ value, label: value })),
  resolutions: [
    { value: '2MP', label: '2MP' },
    { value: '4MP', label: '4MP' },
  ],
  pricing: createFixedResolutionPricing({
    currency: 'USD',
    standardRates: {
      '2MP': 0.04,
      '4MP': 0.06,
    },
  }),
  resolveRequest: ({ referenceImageCount }) => ({
    requestModel: APIYI_FLUX2_MAX_MODEL_ID,
    modeLabel: referenceImageCount > 0 ? '编辑模式' : '生成模式',
  }),
};
