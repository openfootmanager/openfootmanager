import { useTranslation } from "react-i18next";
import { Button } from "../ui";
import DashboardModalFrame from "../dashboard/DashboardModalFrame";
import type { ManagerProfile } from "./types";

interface ProfileSaveConfirmProps {
  loadedProfile: ManagerProfile;
  onUpdate: () => void;
  onSaveNew: () => void;
  onSkip: () => void;
  onClose: () => void;
}

export default function ProfileSaveConfirm({
  loadedProfile,
  onUpdate,
  onSaveNew,
  onSkip,
  onClose,
}: ProfileSaveConfirmProps) {
  const { t } = useTranslation();
  const profileName = `${loadedProfile.first_name} ${loadedProfile.last_name}`;

  return (
    <DashboardModalFrame maxWidthClassName="max-w-sm">
      <h3 className="text-lg font-heading font-bold uppercase tracking-wide text-gray-900 dark:text-white">
        {t("managerProfiles.saveConfirm.title")}
      </h3>

      <p className="mt-2 text-sm text-gray-500 dark:text-gray-400">
        {t("managerProfiles.saveConfirm.body", { name: profileName })}
      </p>

      <div className="mt-6 flex flex-col gap-3">
        <Button variant="primary" size="lg" className="w-full" onClick={onUpdate}>
          {t("managerProfiles.saveConfirm.update", { name: profileName })}
        </Button>

        <Button variant="outline" size="lg" className="w-full" onClick={onSaveNew}>
          {t("managerProfiles.saveConfirm.saveNew")}
        </Button>

        <Button variant="ghost" size="lg" className="w-full" onClick={onSkip}>
          {t("managerProfiles.saveConfirm.skip")}
        </Button>
      </div>

      <button
        type="button"
        onClick={onClose}
        className="mt-6 w-full text-sm text-gray-400 dark:text-gray-500 hover:text-gray-600 dark:hover:text-gray-300 transition-colors text-center"
      >
        {t("menu.cancel")}
      </button>
    </DashboardModalFrame>
  );
}
