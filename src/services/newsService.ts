import { invoke } from "@tauri-apps/api/core";
import type { NewsArticle } from "../store/types";

export interface NewsFeed {
  articles: NewsArticle[];
  team_names: Record<string, string>;
  league_name: string | null;
}

export async function fetchNewsFeed(): Promise<NewsFeed> {
  return invoke<NewsFeed>("get_news_feed", { query: {} });
}
