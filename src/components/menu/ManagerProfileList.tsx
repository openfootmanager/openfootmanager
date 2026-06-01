import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Trash2 } from "lucide-react";
import type { ManagerProfile } from "./types";

interface ManagerProfileListProps {
    profiles: ManagerProfile[];
    selectedProfileId?: string;
    onSelect: (profile: ManagerProfile) => void;
    onDelete: (id: string) => void;
}

export default function ManagerProfileList({ profiles, selectedProfileId, onSelect, onDelete }: ManagerProfileListProps) {
    const { t } = useTranslation();
    const [confirmDeleteId, setConfirmDeleteId] = useState<string | null>(null);

    if (profiles.length === 0) {
        return null;
    }

    return (
        <div className="flex flex-col gap-2 max-h-[13.5rem] overflow-y-auto pr-0.5">
            {profiles.map((profile) => {
                const isSelected = profile.id === selectedProfileId;
                return (
                    <div
                        key={profile.id}
                        className={`group relative flex items-center w-full rounded-xl border transition-all duration-200 ${isSelected
                            ? "bg-primary-50 dark:bg-primary-500/10 border-primary-400 dark:border-primary-500 ring-1 ring-primary-400/30"
                            : "bg-white dark:bg-navy-700 border-gray-200 dark:border-navy-600 hover:border-primary-400 dark:hover:border-primary-500 hover:bg-primary-50 dark:hover:bg-navy-600"
                        }`}
                    >
                        <button
                            type="button"
                            onClick={() => { setConfirmDeleteId(null); onSelect(profile); }}
                            className="flex-1 flex items-center gap-2.5 px-3 py-2.5 text-left min-w-0"
                        >
                            <div className="flex-1 flex items-center justify-between min-w-0">
                                <span className={`font-heading font-bold text-sm uppercase tracking-wide truncate ${isSelected ? "text-primary-600 dark:text-primary-400" : "text-gray-900 dark:text-white"}`}>
                                    {profile.first_name} {profile.last_name}
                                </span>
                                <span className="text-xs text-gray-400 dark:text-gray-500 ml-2 shrink-0">
                                    {profile.nationality} &middot; {profile.date_of_birth}
                                </span>
                            </div>
                            <div className={`w-4 h-4 rounded-full bg-primary-500 flex items-center justify-center shrink-0 ${isSelected ? "visible" : "invisible"}`}>
                                <div className="w-1.5 h-1.5 rounded-full bg-white" />
                            </div>
                        </button>

                        <button
                            type="button"
                            aria-label={t("menu.delete")}
                            onClick={() => setConfirmDeleteId(profile.id)}
                            className="p-1.5 mr-2 rounded-lg text-gray-300 dark:text-gray-600 hover:text-red-500 dark:hover:text-red-400 hover:bg-red-50 dark:hover:bg-red-500/10 transition-all shrink-0"
                        >
                            <Trash2 className="w-3.5 h-3.5" />
                        </button>

                        {confirmDeleteId === profile.id && (
                            <div className={`absolute inset-0 flex items-center justify-end gap-1.5 px-3 rounded-xl ${isSelected ? "bg-primary-50 dark:bg-primary-500/10" : "bg-white dark:bg-navy-700"}`}>
                                <button
                                    type="button"
                                    onClick={() => { onDelete(profile.id); setConfirmDeleteId(null); }}
                                    className="px-2.5 py-1 bg-red-500 hover:bg-red-600 text-white text-xs font-heading font-bold uppercase tracking-wider rounded-lg transition-colors"
                                >
                                    {t("menu.delete")}
                                </button>
                                <button
                                    type="button"
                                    onClick={() => setConfirmDeleteId(null)}
                                    className="px-2.5 py-1 bg-gray-200 hover:bg-gray-300 dark:bg-navy-800 dark:hover:bg-navy-900 text-gray-700 dark:text-gray-300 text-xs font-heading font-bold uppercase tracking-wider rounded-lg transition-colors"
                                >
                                    {t("menu.cancel")}
                                </button>
                            </div>
                        )}
                    </div>
                );
            })}
        </div>
    );
}
