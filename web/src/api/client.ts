const API_BASE = "/api";

export interface TemplateResponse {
  template_address: string;
  template_name: string;
  author_public_key: string;
  author_friendly_name: string | null;
  binary_hash: string;
  at_epoch: number;
  metadata_hash: string | null;
  definition: TemplateDef | null;
  code_size: number | null;
  is_featured: boolean;
  metadata: MetadataResponse | null;
}

export interface MetadataResponse {
  name: string;
  version: string;
  description: string;
  tags: string[];
  category: string | null;
  repository: string | null;
  documentation: string | null;
  homepage: string | null;
  license: string | null;
  logo_url: string | null;
}

export interface TemplateDef {
  V1: {
    template_name: string;
    abi_version: number;
    functions: FunctionDef[];
  };
}

export interface FunctionDef {
  name: string;
  arguments: ArgDef[];
  output: TypeDef;
  is_mut: boolean;
}

export interface ArgDef {
  name: string;
  arg_type: TypeDef;
}

export type TypeDef =
  | "Unit"
  | "Bool"
  | "I8"
  | "I16"
  | "I32"
  | "I64"
  | "I128"
  | "U8"
  | "U16"
  | "U32"
  | "U64"
  | "U128"
  | "String"
  | { Vec: TypeDef }
  | { Tuple: TypeDef[] }
  | { Other: { name: string } }
  | { Option: TypeDef };

export interface SearchResult {
  template_address: string;
  template_name: string;
  author_public_key: string;
  author_friendly_name: string | null;
  at_epoch: number;
  code_size: number | null;
  is_featured: boolean;
  meta_name: string | null;
  meta_description: string | null;
  meta_tags: string[] | null;
  meta_category: string | null;
  meta_logo_url: string | null;
  has_metadata: boolean;
}

export interface SearchResponse {
  results: SearchResult[];
}

export interface AdminTemplate {
  template_address: string;
  template_name: string;
  at_epoch: number;
  is_featured: boolean;
  is_blacklisted: boolean;
  feature_order: number | null;
  has_definition: boolean;
  has_metadata_hash: boolean;
  logo_url: string | null;
}

export interface StatsResponse {
  total_templates: number;
  with_metadata: number;
  with_definition: number;
  featured: number;
  blacklisted: number;
}

export interface AdminUser {
  id: number;
  username: string;
  created_at: string;
}

async function request<T>(path: string, options?: RequestInit): Promise<T> {
  const resp = await fetch(`${API_BASE}${path}`, {
    ...options,
    headers: {
      "Content-Type": "application/json",
      ...getAuthHeaders(),
      ...options?.headers,
    },
  });
  if (!resp.ok) {
    if (resp.status === 401) {
      localStorage.removeItem("admin_token");
      window.location.href = "/admin/login";
      throw new Error("Session expired");
    }
    const body = await resp.json().catch(() => ({ error: resp.statusText }));
    throw new Error(body.error || resp.statusText);
  }
  return resp.json();
}

function getAuthHeaders(): Record<string, string> {
  const token = localStorage.getItem("admin_token");
  return token ? { Authorization: `Bearer ${token}` } : {};
}

export function isLoggedIn(): boolean {
  return !!localStorage.getItem("admin_token");
}

export function logout() {
  localStorage.removeItem("admin_token");
}

// Public API
export const api = {
  getFeatured: () => request<TemplateResponse[]>("/templates/featured"),

  getTemplate: (addr: string) => request<TemplateResponse>(`/templates/${addr}`),

  search: (params: {
    q?: string;
    tags?: string;
    category?: string;
    author?: string;
    limit?: number;
    offset?: number;
  }) => {
    const qs = new URLSearchParams();
    if (params.q) qs.set("q", params.q);
    if (params.tags) qs.set("tags", params.tags);
    if (params.category) qs.set("category", params.category);
    if (params.author) qs.set("author", params.author);
    if (params.limit) qs.set("limit", String(params.limit));
    if (params.offset) qs.set("offset", String(params.offset));
    return request<SearchResponse>(`/search?${qs}`);
  },

  // Auth
  login: (username: string, password: string) =>
    request<{ token: string }>("/auth/login", {
      method: "POST",
      body: JSON.stringify({ username, password }),
    }),

  // Admin
  admin: {
    getStats: () => request<StatsResponse>("/admin/stats"),

    listTemplates: (limit = 50, offset = 0) =>
      request<AdminTemplate[]>(`/admin/templates?limit=${limit}&offset=${offset}`),

    setFeatured: (addr: string, featured: boolean, order?: number) =>
      request<{ ok: boolean }>(`/admin/templates/${addr}/featured`, {
        method: "PUT",
        body: JSON.stringify({ featured, order }),
      }),

    setBlacklisted: (addr: string, blacklisted: boolean) =>
      request<{ ok: boolean }>(`/admin/templates/${addr}/blacklist`, {
        method: "PUT",
        body: JSON.stringify({ blacklisted }),
      }),

    listAdmins: () => request<AdminUser[]>("/admin/admins"),

    createAdmin: (username: string, password: string) =>
      request<AdminUser>("/admin/admins", {
        method: "POST",
        body: JSON.stringify({ username, password }),
      }),

    deleteAdmin: (id: number) =>
      request<{ ok: boolean }>(`/admin/admins/${id}`, { method: "DELETE" }),

    changePassword: (id: number, new_password: string) =>
      request<{ ok: boolean }>(`/admin/admins/${id}/password`, {
        method: "PUT",
        body: JSON.stringify({ new_password }),
      }),
  },
};
