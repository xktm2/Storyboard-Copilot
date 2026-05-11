# APIYI 中转站模型接入参考手册

> 基于本系统（AssetsManager）实际运行代码 + APIYI 官方文档整理，供其他系统接入时参考。
> 数据来源：`backend/app/generation/config/` 配置 + `backend/app/generation/providers/` 适配器代码 + APIYI 文档

---

## 一、供应商通用信息

五模型共用同一中转站 **APIYI**，同一对密钥：

| 项目 | 值 |
|---|---|
| Base URL 环境变量 | `OPENAI_COMPAT_BASE_URL` |
| API Key 环境变量 | `OPENAI_COMPAT_API_KEY` |
| 认证方式 | `Authorization: Bearer {api_key}` |
| 密钥是否加密 | 否（明文存储） |
| 协议风格 | 非统一，分属三种协议族（见下文） |

---

## 二、五模型速查总表

| 维度 | FLUX.2 Max | Nano Banana 2 | Nano Banana Pro | GPT Image 2 | Seedream 5.0 |
|---|---|---|---|---|---|
| **API model 名** | `flux-2-max` | `gemini-3.1-flash-image-preview` | `gemini-3-pro-image-preview` | `gpt-image-2` | `seedream-5-0-260128` |
| **API model 别名** | — | — | — | — | `seedream-5-0-lite-260128` |
| **本系统 provider_id** | `apiyi_flux` | `apiyi_gemini` | 待定 | `apiyi_gpt_image` | 待定 |
| **协议族** | OpenAI Images | Gemini 原生 | Gemini 原生 | OpenAI Images | OpenAI Images 变体 |
| **文生图端点** | `POST /v1/images/generations` | `POST /v1beta/models/{model}:generateContent` | `POST /v1beta/models/{model}:generateContent` | `POST /v1/images/generations` | `POST /v1/images/generations` |
| **图生图端点** | `POST /v1/images/edits` | 同上 | 同上 | `POST /v1/images/edits` | 同文生图端点 |
| **超时** | 180s | 300s | 300s | 360s | 建议 300s |
| **支持文生图** | 是 | 是 | 是 | 是 | 是 |
| **支持图生图** | 是（最多 8 张） | 是（最多 14 张） | 是（多图） | 是（最多 16 张） | 是（最多 10 张） |
| **支持批量序列** | 否 | 否 | 否 | 否 | 是 |
| **最大 prompt** | 32000 字符 | 2000 字符 | 未明确 | 32000 字符 | 未明确 |
| **单次输出** | 1 张 | 1 张 | 1 张 | 1 张 | 1~15 张 |
| **分辨率档位** | 2MP, 4MP | 512, 1K, 2K, 4K | 1K, 2K, 4K | 1K, 2K, 4K | 2K, 3K |
| **宽高比数量** | 8 种 | 14 种 | 10 种 | 8 种 | 8 种 |

---

## 三、FLUX.2 Max 详细规格

### 3.1 基本信息

- **API model 名**: `flux-2-max`
- **显示名**: FLUX.2 Max
- **描述**: 旗舰画质，支持联网搜索与4MP输出，最多8张参考图融合

### 3.2 支持比例与分辨率

#### 推荐分辨率 2MP

| 比例 | 像素尺寸 |
|---|---|
| 1:1 | 1440 × 1440 |
| 4:3 | 1600 × 1200 |
| 3:4 | 1200 × 1600 |
| 16:9 | 1920 × 1080 |
| 9:16 | 1080 × 1920 |
| 3:2 | 1536 × 1024 |
| 2:3 | 1024 × 1536 |
| 21:9 | 2240 × 960 |

#### 高清分辨率 4MP

| 比例 | 像素尺寸 |
|---|---|
| 1:1 | 2048 × 2048 |
| 4:3 | 2304 × 1728 |
| 3:4 | 1728 × 2304 |
| 16:9 | 2560 × 1440 |
| 9:16 | 1440 × 2560 |
| 3:2 | 2304 × 1536 |
| 2:3 | 1536 × 2304 |
| 21:9 | 2912 × 1248 |

#### 自定义尺寸

- 像素范围: 4096 ~ 4,194,304 (总像素)
- 宽高比范围: 0.0625 ~ 16
- 步进: 16px
- 默认: 1024 × 1024

### 3.3 文生图请求

```
POST {base_url}/v1/images/generations
Authorization: Bearer {api_key}
Content-Type: application/json

{
  "model": "flux-2-max",
  "prompt": "一只猫",
  "n": 1,
  "size": "1440x1440",
  "output_format": "jpeg"
}
```

**特有参数**:
- `output_format`: 输出格式，默认 `"jpeg"`

### 3.4 图生图请求

```
POST {base_url}/v1/images/edits
Authorization: Bearer {api_key}
Content-Type: multipart/form-data

model: flux-2-max
prompt: 一只猫
n: 1
output_format: jpeg
image: <第1张参考图文件上传>
input_image_2: <第2张参考图的 URL 字符串>
input_image_3: <第3张参考图的 URL 字符串>
...
```

**参考图传递方式（重要差异）**:
- **第 1 张**: multipart 文件上传，字段名 `image`
- **第 2~8 张**: 以 URL 字符串放在 form data 中，字段名 `input_image_2`, `input_image_3`, ...

### 3.5 响应格式

```json
{
  "data": [
    { "url": "https://...临时URL，约10分钟有效" }
  ]
}
```

- 返回临时 URL，需**二次下载**获取图片内容
- URL 约 10 分钟过期

---

## 四、Nano Banana 2 详细规格

### 4.1 基本信息

- **API model 名**: `gemini-3.1-flash-image-preview`
- **显示名**: Nano Banana 2
- **描述**: 支持文生图与多参考图编辑，14种宽高比、4档分辨率及深度推理模式

### 4.2 支持比例与分辨率

支持 4 档分辨率 × 14 种比例：

#### 512px

| 比例 | 像素尺寸 | | 比例 | 像素尺寸 |
|---|---|---|---|---|
| 1:1 | 512×512 | | 4:5 | 410×512 |
| 4:3 | 512×384 | | 5:4 | 512×410 |
| 3:4 | 384×512 | | 21:9 | 512×220 |
| 3:2 | 512×341 | | 1:4 | 128×512 |
| 2:3 | 341×512 | | 4:1 | 512×128 |
| 16:9 | 512×288 | | 1:8 | 64×512 |
| 9:16 | 288×512 | | 8:1 | 512×64 |

#### 1K

| 比例 | 像素尺寸 | | 比例 | 像素尺寸 |
|---|---|---|---|---|
| 1:1 | 1024×1024 | | 4:5 | 820×1024 |
| 4:3 | 1024×768 | | 5:4 | 1024×820 |
| 3:4 | 768×1024 | | 21:9 | 1024×439 |
| 3:2 | 1024×683 | | 1:4 | 256×1024 |
| 2:3 | 683×1024 | | 4:1 | 1024×256 |
| 16:9 | 1024×576 | | 1:8 | 128×1024 |
| 9:16 | 576×1024 | | 8:1 | 1024×128 |

#### 2K

| 比例 | 像素尺寸 | | 比例 | 像素尺寸 |
|---|---|---|---|---|
| 1:1 | 2048×2048 | | 4:5 | 1638×2048 |
| 4:3 | 2048×1536 | | 5:4 | 2048×1638 |
| 3:4 | 1536×2048 | | 21:9 | 2048×878 |
| 3:2 | 2048×1365 | | 1:4 | 512×2048 |
| 2:3 | 1365×2048 | | 4:1 | 2048×512 |
| 16:9 | 2048×1152 | | 1:8 | 256×2048 |
| 9:16 | 1152×2048 | | 8:1 | 2048×256 |

#### 4K

| 比例 | 像素尺寸 | | 比例 | 像素尺寸 |
|---|---|---|---|---|
| 1:1 | 4096×4096 | | 4:5 | 3277×4096 |
| 4:3 | 4096×3072 | | 5:4 | 4096×3277 |
| 3:4 | 3072×4096 | | 21:9 | 4096×1756 |
| 3:2 | 4096×2731 | | 1:4 | 1024×4096 |
| 2:3 | 2731×4096 | | 4:1 | 4096×1024 |
| 16:9 | 4096×2304 | | 1:8 | 512×4096 |
| 9:16 | 2304×4096 | | 8:1 | 4096×512 |

### 4.3 文生图请求

```
POST {base_url}/v1beta/models/gemini-3.1-flash-image-preview:generateContent
Authorization: Bearer {api_key}
Content-Type: application/json

{
  "contents": [
    {
      "parts": [
        { "text": "一只猫" }
      ]
    }
  ],
  "generationConfig": {
    "responseModalities": ["TEXT", "IMAGE"],
    "imageConfig": {
      "imageSize": "1K",
      "aspectRatio": "1:1"
    },
    "thinkingConfig": {
      "thinkingLevel": "minimal"
    }
  }
}
```

**generationConfig.imageConfig 字段说明**:
- `imageSize`: 分辨率档位，可选 `"512"`, `"1K"`, `"2K"`, `"4K"`
- `aspectRatio`: 宽高比，可选值见上表（如 `"1:1"`, `"16:9"` 等），`"1:1"` 时可省略

**generationConfig.thinkingConfig 字段说明**:
- `thinkingLevel`: 推理深度，可选 `"minimal"`（快速）或 `"High"`（深度推理）

### 4.4 图生图请求

```
POST {base_url}/v1beta/models/gemini-3.1-flash-image-preview:generateContent
Authorization: Bearer {api_key}
Content-Type: application/json

{
  "contents": [
    {
      "parts": [
        { "text": "一只猫" },
        { "fileData": { "fileUri": "https://...图片URL", "mimeType": "image/png" } },
        { "fileData": { "fileUri": "https://...图片URL", "mimeType": "image/jpeg" } }
      ]
    }
  ],
  "generationConfig": { ... }
}
```

**参考图传递方式（重要 — 双层降级机制）**:

1. **优先方式 — fileData（URL 传参）**:
   - 所有参考图使用 `{ "fileData": { "fileUri": "url", "mimeType": "mime" } }`
   - URL 必须是 Gemini 服务器可公网访问的地址

2. **降级方式 — inlineData（base64 内联）**:
   - 当 Gemini 返回 `"cannot fetch content from the provided url"` 错误时自动降级
   - 改用 `{ "inlineData": { "data": "base64编码", "mimeType": "image/png" } }`
   - 本系统在适配器中实现了自动降级逻辑

### 4.5 响应格式

```json
{
  "candidates": [
    {
      "content": {
        "parts": [
          { "text": "可能的文本回复" },
          {
            "inlineData": {
              "data": "base64编码的图片数据",
              "mimeType": "image/png"
            }
          }
        ]
      }
    }
  ],
  "usageMetadata": { ... }
}
```

- 图片**内嵌在响应中**（base64），无需二次下载
- 响应中可能同时包含 `text` 部分，需遍历 parts 找 `inlineData`

---

## 五、Nano Banana Pro 详细规格

### 5.1 基本信息

- **API model 名**: `gemini-3-pro-image-preview`
- **显示名**: Nano Banana Pro
- **描述**: 谷歌最强图像生成模型，支持 4K 高清输出、业界最佳文字渲染、高级局部编辑
- **上线日期**: 2025-11-20
- **定价**: $0.09/张（约¥0.52）

### 5.2 与 Nano Banana 2 的差异

Nano Banana Pro 与 Nano Banana 2 **同属 Gemini 协议族**，端点、请求/响应结构相同，但有如下关键差异：

| 维度 | Nano Banana 2 | Nano Banana Pro |
|---|---|---|
| API model 名 | `gemini-3.1-flash-image-preview` | `gemini-3-pro-image-preview` |
| 画质定位 | Pro 级画质 + Flash 级速度 | 极致画质 |
| 生成速度 | ~10s | ~20s |
| 分辨率档位 | 512, 1K, 2K, 4K（4档） | 1K, 2K, 4K（3档，无 512px） |
| 宽高比 | 14 种（含 1:4, 4:1, 1:8, 8:1） | 10 种（无极端比例） |
| thinkingConfig | 支持（`minimal` / `High`） | **不支持** |
| Google Search Grounding | 支持 | **不支持** |
| 参考图传递 | 优先 `fileData` URL，降级 `inlineData` base64 | **仅 `inlineData` base64** |
| 定价 | $0.025/张 | $0.09/张 |

### 5.3 支持比例与分辨率

支持 3 档分辨率 × 10 种比例：

#### 1K

| 比例 | 像素尺寸 |
|---|---|
| 1:1 | 1024 × 1024 |
| 4:3 | 1024 × 768 |
| 3:4 | 768 × 1024 |
| 3:2 | 1024 × 683 |
| 2:3 | 683 × 1024 |
| 16:9 | 1024 × 576 |
| 9:16 | 576 × 1024 |
| 4:5 | 820 × 1024 |
| 5:4 | 1024 × 820 |
| 21:9 | 1024 × 439 |

#### 2K

| 比例 | 像素尺寸 |
|---|---|
| 1:1 | 2048 × 2048 |
| 4:3 | 2048 × 1536 |
| 3:4 | 1536 × 2048 |
| 3:2 | 2048 × 1365 |
| 2:3 | 1365 × 2048 |
| 16:9 | 2048 × 1152 |
| 9:16 | 1152 × 2048 |
| 4:5 | 1638 × 2048 |
| 5:4 | 2048 × 1638 |
| 21:9 | 2048 × 878 |

#### 4K

| 比例 | 像素尺寸 |
|---|---|
| 1:1 | 4096 × 4096 |
| 4:3 | 4096 × 3072 |
| 3:4 | 3072 × 4096 |
| 3:2 | 4096 × 2731 |
| 2:3 | 2731 × 4096 |
| 16:9 | 4096 × 2304 |
| 9:16 | 2304 × 4096 |
| 4:5 | 3277 × 4096 |
| 5:4 | 4096 × 3277 |
| 21:9 | 4096 × 1756 |

### 5.4 文生图请求

```
POST {base_url}/v1beta/models/gemini-3-pro-image-preview:generateContent
Authorization: Bearer {api_key}
Content-Type: application/json

{
  "contents": [
    {
      "parts": [
        { "text": "一只猫" }
      ]
    }
  ],
  "generationConfig": {
    "responseModalities": ["IMAGE"],
    "imageConfig": {
      "imageSize": "2K",
      "aspectRatio": "16:9"
    }
  }
}
```

**与 NB2 的差异**:
- `responseModalities`: 可用 `["IMAGE"]`（仅图片）或 `["TEXT", "IMAGE"]`（图片+文本）
- **不能传 `thinkingConfig`** — NB Pro 不支持，传入可能导致错误或被忽略

### 5.5 图生图请求

```
POST {base_url}/v1beta/models/gemini-3-pro-image-preview:generateContent
Authorization: Bearer {api_key}
Content-Type: application/json

{
  "contents": [
    {
      "parts": [
        { "text": "请把背景模糊化，突出前景的人物" },
        { "inlineData": { "mimeType": "image/jpeg", "data": "<BASE64_DATA_IMG_1>" } },
        { "inlineData": { "mimeType": "image/png", "data": "<BASE64_DATA_IMG_2>" } }
      ]
    }
  ],
  "generationConfig": {
    "responseModalities": ["IMAGE"],
    "imageConfig": {
      "imageSize": "2K",
      "aspectRatio": "16:9"
    }
  }
}
```

**参考图传递方式（与 NB2 的重要差异）**:

- **Nano Banana Pro 使用 `inlineData`（base64 内联）**，不是 `fileData`（URL）
- 每个 part 只能含 `text` 或 `inlineData` 之一，不可同时出现
- 多图编辑：1 个 text part（编辑指令）+ N 个 inlineData part（每张图一个）
- 图片需先读取并 base64 编码后放入请求体

### 5.6 响应格式

与 NB2 完全相同：

```json
{
  "candidates": [
    {
      "content": {
        "parts": [
          {
            "inlineData": {
              "data": "base64编码的图片数据",
              "mimeType": "image/png"
            }
          }
        ]
      },
      "finishReason": "STOP"
    }
  ],
  "usageMetadata": {
    "promptTokenCount": 10,
    "candidatesTokenCount": 258
  }
}
```

- 图片**内嵌在响应中**（base64），无需二次下载
- **内容审核拒绝判断**: `candidatesTokenCount` 为 0 表示审核阶段被拒；`finishReason` 非 `STOP`（如 `PROHIBITED_CONTENT`、`SAFETY`）表示生成中被拒；响应可能返回文本说明而非图片

---

## 六、GPT Image 2 详细规格

### 6.1 基本信息

- **API model 名**: `gpt-image-2`
- **显示名**: GPT Image 2
- **描述**: OpenAI GPT Image 2，支持文生图与多参考图编辑

### 6.2 支持比例与分辨率

#### 1K

| 比例 | 像素尺寸 |
|---|---|
| 1:1 | 1024 × 1024 |
| 4:3 | 1152 × 864 |
| 3:4 | 864 × 1152 |
| 3:2 | 1536 × 1024 |
| 2:3 | 1024 × 1536 |
| 16:9 | 1792 × 1008 |
| 9:16 | 1008 × 1792 |
| 21:9 | 1568 × 672 |

#### 2K

| 比例 | 像素尺寸 |
|---|---|
| 1:1 | 2048 × 2048 |
| 4:3 | 2304 × 1728 |
| 3:4 | 1728 × 2304 |
| 3:2 | 2496 × 1664 |
| 2:3 | 1664 × 2496 |
| 16:9 | 2048 × 1152 |
| 9:16 | 1152 × 2048 |
| 21:9 | 2912 × 1248 |

#### 4K

| 比例 | 像素尺寸 |
|---|---|
| 1:1 | 2880 × 2880 |
| 4:3 | 3264 × 2448 |
| 3:4 | 2448 × 3264 |
| 3:2 | 3504 × 2336 |
| 2:3 | 2336 × 3504 |
| 16:9 | 3840 × 2160 |
| 9:16 | 2160 × 3840 |
| 21:9 | 3808 × 1632 |

#### 自定义尺寸

- 像素范围: 655,360 ~ 8,294,400 (总像素)
- 宽高比范围: 0.333 ~ 3
- 步进: 16px
- 默认: 1024 × 1024

### 6.3 文生图请求

```
POST {base_url}/v1/images/generations
Authorization: Bearer {api_key}
Content-Type: application/json

{
  "model": "gpt-image-2",
  "prompt": "一只猫",
  "n": 1,
  "size": "1024x1024",
  "quality": "auto"
}
```

**特有参数**:
- `quality`: 图片质量，可选 `"auto"`（默认）、`"low"`、`"medium"`、`"high"`

### 6.4 图生图请求

```
POST {base_url}/v1/images/edits
Authorization: Bearer {api_key}
Content-Type: multipart/form-data

model: gpt-image-2
prompt: 一只猫
size: 1024x1024
quality: auto
image[]: <参考图1文件上传>
image[]: <参考图2文件上传>
...
```

**参考图传递方式**:
- 所有参考图均通过 **multipart 文件上传**，字段名统一为 `image[]`（数组风格）
- 最多 16 张参考图，单张最大 10MB

### 6.5 响应格式

```json
{
  "data": [
    { "b64_json": "base64编码的图片数据" }
  ]
}
```

或

```json
{
  "data": [
    { "url": "https://...临时URL" }
  ]
}
```

- **优先解析 `b64_json`**（base64 内嵌）
- 若无 b64_json 则 fallback 到 `url`，需二次下载
- 响应中可能包含 `usage` 字段

---

## 七、Seedream 5.0 详细规格

### 7.1 基本信息

- **API model 名**: `seedream-5-0-260128`
- **API model 别名**: `seedream-5-0-lite-260128`
- **显示名**: Seedream 5.0
- **描述**: 字节火山方舟 Seedream 5.0，支持文生图、单图编辑、多图融合与批量序列生成
- **上线日期**: 2026-01-28

### 7.2 支持比例与分辨率

#### 预设分辨率

| 档位 | 像素基准 | 说明 |
|---|---|---|
| 2K | 约 2048×2048 | 默认 |
| 3K | 约 3072×3072 | 仅 5.0 支持 |

预设档位下会自动按比例分配宽高。各比例的精确像素值参考本系统现有 Seedream 5.0 配置：

##### 2K

| 比例 | 像素尺寸 |
|---|---|
| 1:1 | 2048 × 2048 |
| 4:3 | 2304 × 1728 |
| 3:4 | 1728 × 2304 |
| 16:9 | 2848 × 1600 |
| 9:16 | 1600 × 2848 |
| 3:2 | 2496 × 1664 |
| 2:3 | 1664 × 2496 |
| 21:9 | 3136 × 1344 |

##### 3K

| 比例 | 像素尺寸 |
|---|---|
| 1:1 | 3072 × 3072 |
| 4:3 | 3456 × 2592 |
| 3:4 | 2592 × 3456 |
| 16:9 | 4096 × 2304 |
| 9:16 | 2304 × 4096 |
| 3:2 | 3744 × 2496 |
| 2:3 | 2496 × 3744 |
| 21:9 | 4704 × 2016 |

#### 自定义尺寸

- 总像素范围: [1280×720, 4096×4096]
- 宽高比范围: [1/16, 16]
- 格式: `"WxH"`（如 `"1920x1080"`）

### 7.3 文生图请求

```
POST {base_url}/v1/images/generations
Authorization: Bearer {api_key}
Content-Type: application/json

{
  "model": "seedream-5-0-260128",
  "prompt": "一只猫",
  "size": "2K",
  "response_format": "url",
  "output_format": "jpeg",
  "watermark": false
}
```

**特有参数**:
- `output_format`: 输出格式，5.0 支持 `"png"` 和 `"jpeg"`（默认 `"jpeg"`）
- `watermark`: 是否加水印，默认 `false`
- `response_format`: `"url"`（默认）或 `"b64_json"`
- `stream`: 流式输出，默认 `false`

### 7.4 图生图请求

```
POST {base_url}/v1/images/generations
Authorization: Bearer {api_key}
Content-Type: application/json

{
  "model": "seedream-5-0-260128",
  "prompt": "Replace the clothing in image 1 with the outfit from image 2.",
  "image": [
    "https://your-oss.example.com/person.png",
    "https://your-oss.example.com/outfit.png"
  ],
  "sequential_image_generation": "disabled",
  "size": "2K",
  "response_format": "url",
  "output_format": "jpeg",
  "watermark": false
}
```

**关键差异 — 与其他模型完全不同的参考图传递方式**:
- **没有 `/v1/images/edits` 端点**，文生图和图生图都走 `/v1/images/generations`
- **不接受 multipart/form-data**，所有内容通过 JSON body 传递
- 参考图通过 `image` 字段（URL 数组）传入，不是文件上传
- `image` 数组最多 10 张 URL
- `sequential_image_generation`: `"disabled"` 表示单图输出（默认）

### 7.5 批量序列生成请求

```
POST {base_url}/v1/images/generations
Authorization: Bearer {api_key}
Content-Type: application/json

{
  "model": "seedream-5-0-260128",
  "prompt": "Generate four storyboard scenes: ...",
  "sequential_image_generation": "auto",
  "sequential_image_generation_options": { "max_images": 4 },
  "size": "2K",
  "response_format": "url",
  "output_format": "jpeg",
  "watermark": false
}
```

**批量序列特有参数**:
- `sequential_image_generation`: `"auto"` 开启批量序列模式
- `sequential_image_generation_options.max_images`: 最大输出张数
- **总约束**: 输入参考图 + 输出图 ≤ 15 张

### 7.6 响应格式

```json
{
  "model": "seedream-5-0-260128",
  "created": 1768518000,
  "data": [
    {
      "url": "https://ark-content-generation-v2-ap-southeast-1.tos-ap-southeast-1.bytepluses.com/.../scene-1.png",
      "size": "2048x2048"
    },
    {
      "url": "https://...scene-2.png",
      "size": "2048x2048"
    }
  ],
  "usage": {
    "generated_images": 2,
    "output_tokens": 12480,
    "total_tokens": 12480
  }
}
```

- `response_format: "url"` 时返回 `data[].url`（临时 URL）
- `response_format: "b64_json"` 时返回 `data[].b64_json`（base64 内嵌）
- 批量序列模式 `data` 数组含多个元素，单图模式只含 1 个
- 计费按 `usage.generated_images` 实际出图张数，不按 `max_images`

---

## 八、五模型请求格式差异汇总

### 协议族分类

| 协议族 | 模型 | 端点风格 | 参考图方式 |
|---|---|---|---|
| **OpenAI Images** | FLUX.2 Max, GPT Image 2 | generations + edits 分离 | multipart 文件上传 |
| **Gemini 原生** | Nano Banana 2, Nano Banana Pro | generateContent 统一入口 | JSON 内嵌（URL 或 base64） |
| **OpenAI Images 变体** | Seedream 5.0 | generations 统一入口（无 edits） | JSON URL 数组 |

### 文生图

| 模型 | 请求格式 | 尺寸传递 | 特有参数 |
|---|---|---|---|
| FLUX.2 Max | JSON body | `size: "1440x1440"` | `output_format` |
| Nano Banana 2 | JSON body (Gemini 格式) | `imageSize` + `aspectRatio` | `thinkingConfig.thinkingLevel` |
| Nano Banana Pro | JSON body (Gemini 格式) | `imageSize` + `aspectRatio` | 无 |
| GPT Image 2 | JSON body | `size: "1024x1024"` | `quality` |
| Seedream 5.0 | JSON body | `size: "2K"` 或 `"2048x2048"` | `output_format`, `watermark`, `stream`, `sequential_image_generation` |

### 图生图 — 参考图传递方式（最关键差异）

| 模型 | 参考图格式 | Content-Type | 端点 |
|---|---|---|---|
| **FLUX.2 Max** | 第1张 multipart `image` 上传，其余 URL `input_image_2`... | multipart/form-data | `/v1/images/edits` |
| **Nano Banana 2** | JSON `fileData.fileUri`（URL），降级 `inlineData`（base64） | application/json | `/v1beta/models/{model}:generateContent` |
| **Nano Banana Pro** | JSON `inlineData`（base64 内联） | application/json | `/v1beta/models/{model}:generateContent` |
| **GPT Image 2** | multipart `image[]` 数组上传 | multipart/form-data | `/v1/images/edits` |
| **Seedream 5.0** | JSON `image: ["url"]`（URL 数组） | application/json | `/v1/images/generations` |

### 响应格式

| 模型 | 图片位置 | 是否需二次下载 |
|---|---|---|
| **FLUX.2 Max** | `data[].url`（临时 URL） | 是 |
| **Nano Banana 2** | `candidates[].content.parts[].inlineData.data`（base64） | 否 |
| **Nano Banana Pro** | `candidates[].content.parts[].inlineData.data`（base64） | 否 |
| **GPT Image 2** | `data[].b64_json`（base64）或 `data[].url`（URL） | b64_json 否，URL 是 |
| **Seedream 5.0** | `data[].url` 或 `data[].b64_json`（由 `response_format` 决定） | url 是，b64_json 否 |

---

## 九、接入注意事项

1. **三种协议族**: OpenAI Images（FLUX/GPT Image 2，有独立 edits 端点），Gemini 原生（NB2/NB Pro，generateContent 统一入口），OpenAI Images 变体（Seedream 5.0，generations 统一入口但无 edits）。

2. **NB2 vs NB Pro 参考图方式不同**: NB2 优先用 `fileData.fileUri`（URL 传参），URL 不可达时降级 `inlineData`（base64）；NB Pro **直接用 `inlineData`（base64）**，不经过 URL 尝试。接入时若两个模型共用适配器，需根据 model 区分参考图构建逻辑。

3. **NB Pro 不支持 thinkingConfig**: 传入可能导致错误或被忽略，与 NB2 的适配器逻辑需做区分。

4. **NB Pro 内容审核更严格**: 需额外处理 `candidatesTokenCount: 0`（审核阶段拒绝）和 `finishReason` 非 `STOP`（生成中拒绝）两种情况。

5. **Seedream 5.0 无 edits 端点**: 文生图和图生图都走 `POST /v1/images/generations`，图生图通过请求体中的 `image` URL 数组 + `sequential_image_generation: "disabled"` 切换。不要用 `/v1/images/edits`。

6. **Seedream 5.0 不接受 multipart**: 参考图必须先上传到公网可访问的 URL，再通过 JSON body 的 `image` 数组传入。

7. **Seedream 5.0 张数约束**: 输入参考图 + 输出图总数 ≤ 15。`sequential_image_generation: auto` 模式下尤其要注意。

8. **NB2 URL 降级**: Gemini 的 `fileData.fileUri` 要求图片 URL 对 Gemini 服务器可公网访问。如果图片存储在内网（如本地 storage），需准备 base64 inlineData 降级方案。

9. **FLUX 参考图混合传参**: 第 1 张 multipart 上传，其余 URL 字符串。

10. **FLUX 临时 URL 有效期**: 返回的 URL 约 10 分钟过期，需及时下载。

11. **GPT Image 2 响应双格式**: 可能返回 b64_json 也可能返回 url，需同时处理两种情况。

12. **size 参数格式**: FLUX 和 GPT Image 2 用 `"宽x高"` 格式（如 `"1024x1024"`），NB2/NB Pro 用 `imageSize` + `aspectRatio` 分开传递，Seedream 5.0 支持档位名（`"2K"`）或精确像素（`"2048x2048"`）。

13. **Seedream 5.0 批量序列**: 五模型中唯一原生支持批量序列生成。其他模型需客户端重复调用。
