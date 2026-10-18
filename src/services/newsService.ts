import { invoke } from "@tauri-apps/api/core";
import type { NewsArticle } from "../store/types";

export interface NewsFeed {
  articles: NewsArticle[];
  team_names: Record<string, string>;
  league_name: string | null;
  league_name_key?: string | null;
  league_country_id?: string | null;
  league_division_tier?: number | null;
}

export async function fetchNewsFeed(): Promise<NewsFeed> {
  return invoke<NewsFeed>("get_news_feed", { query: {} });
}
