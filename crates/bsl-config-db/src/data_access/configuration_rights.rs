//! Проверка достигнутых прав конфигурации и чтения владельцев по ресурсам ролей.

use super::*;
use bsl_orm::{AccessRightRequest, AccessRightSubject};
use open_sdbl::metadata::Right;

pub(super) fn metadata_access_right(
    access: &AccessStore,
    snapshot: &MetadataSnapshot,
    request: &AccessRightRequest,
    principal: &Principal,
    context: &mut CallContext<'_>,
    verification: bool,
) -> RtResult<bool> {
    if !matches!(
        request.metadata().kind(),
        MetadataKind::Catalog | MetadataKind::Document
    ) {
        return access_right(access, request, principal, context);
    }
    let owner: Guid = request
        .metadata()
        .id()
        .parse()
        .map_err(|failure| error(format!("неверный идентификатор владельца: {failure}")))?;
    let target = if let Some(attribute) = request.attribute() {
        if request.standard_attribute().is_some() {
            return Err(error(
                "стандартный путь для реквизита документа пока не поддерживается",
            ));
        }
        let child: Guid = attribute
            .id()
            .parse()
            .map_err(|failure| error(format!("неверный идентификатор реквизита: {failure}")))?;
        open_sdbl::metadata::resolve_document_attribute_right(
            snapshot,
            &owner,
            &child,
            request.right(),
        )
    } else {
        open_sdbl::metadata::resolve_metadata_right(
            snapshot,
            &owner,
            request.right(),
            request.standard_attribute(),
        )
    }
    .map_err(|failure| error(failure.to_string()))?;
    if target.applicability() == open_sdbl::metadata::MetadataRightApplicability::NotApplicable {
        return if verification {
            Ok(true)
        } else {
            Err(error(if request.attribute().is_some() {
                "право неприменимо к реквизиту"
            } else {
                "право неприменимо к стандартному реквизиту"
            }))
        };
    }
    // Структура уже проверена SDK. Привилегии обходят только выдачу права,
    // не превращая неизвестный путь или неприменимую пару в разрешение.
    let privileged = context.privileged_mode()?;
    if privileged && matches!(request.subject(), AccessRightSubject::CurrentPrincipal) {
        return Ok(true);
    }
    // Явный субъект сохраняет собственные роли даже в привилегированном
    // кадре. Для описания пользователя отдельно проверяется администрирование;
    // неприменимость права уже установлена выше, как в нативном контроле
    // (`docs/research/fixtures/metadata-explicit-subjects-native-1018/platform.oracle`).
    let root = if !privileged && matches!(request.subject(), AccessRightSubject::Principal(_)) {
        context
            .host_data()?
            .downcast_ref::<SessionData>()
            .and_then(|session| session.catalog().configuration_object_id())
            .map(str::to_owned)
    } else {
        None
    };
    let roles = subject_role_ids(
        access,
        request.subject(),
        principal,
        privileged,
        root.as_deref(),
    )?;
    if verification {
        access.metadata_right_for_verification(&roles, &target)
    } else {
        access.metadata_right(&roles, &target)
    }
    .map_err(|failure| error(failure.to_string()))
}

pub(super) fn access_right(
    access: &AccessStore,
    request: &AccessRightRequest,
    principal: &Principal,
    context: &mut CallContext<'_>,
) -> RtResult<bool> {
    // Реквизит имеет собственную применимость прав; право владельца её не заменяет.
    if request.attribute().is_some() {
        return Err(error(
            "проверка прав реквизита документа не поддерживается адаптером данных",
        ));
    }
    if matches!(
        request.metadata().kind(),
        MetadataKind::Catalog
            | MetadataKind::Document
            | MetadataKind::InformationRegister
            | MetadataKind::AccumulationRegister
            | MetadataKind::Constant
    ) && Right::parse(request.right()) == Some(Right::Read)
        && request.standard_attribute().is_none()
        && matches!(request.subject(), AccessRightSubject::CurrentPrincipal)
    {
        let owner: Guid = request
            .metadata()
            .id()
            .parse()
            .map_err(|failure| error(format!("неверный идентификатор владельца: {failure}")))?;
        // Нативные контроли опций и RLS различают само право чтения и отбор
        // строк. Проверку выдачи права и умолчаний уже выполняет SDK;
        // вычислять условие RLS для ответа ПравоДоступа не требуется.
        // Замер RLS: `docs/research/fixtures/accumulation-field-access/query-rls-allowed.platform.txt`;
        // половина про функциональные опции отдельно не измерена.
        if context.privileged_mode()? {
            return Ok(true);
        }
        return grants(access, &role_ids(principal)?, &owner, &Right::Read);
    }
    if request.metadata().kind() != MetadataKind::Configuration {
        return Err(error(
            "проверка прав метаданных не поддерживается адаптером данных",
        ));
    }
    let right = Right::parse(request.right())
        .filter(|right| matches!(right, Right::SaveUserData | Right::Administration))
        .ok_or_else(|| error("проверка этого права конфигурации не поддерживается"))?;
    if request.standard_attribute().is_some() {
        return Err(error(
            "проверка реквизита корня конфигурации не поддерживается",
        ));
    }
    let root: Guid = request
        .metadata()
        .id()
        .parse()
        .map_err(|failure| error(format!("неверный идентификатор конфигурации: {failure}")))?;
    let privileged = context.privileged_mode()?;
    if privileged && matches!(request.subject(), AccessRightSubject::CurrentPrincipal) {
        return Ok(true);
    }
    let roles = subject_role_ids(
        access,
        request.subject(),
        principal,
        privileged,
        Some(root.as_str()),
    )?;
    grants(access, &roles, &root, &right)
}

fn subject_role_ids(
    access: &AccessStore,
    subject: &AccessRightSubject,
    principal: &Principal,
    privileged: bool,
    root: Option<&str>,
) -> RtResult<Vec<Guid>> {
    Ok(match subject {
        AccessRightSubject::CurrentPrincipal => role_ids(principal)?,
        AccessRightSubject::Role(id) => vec![
            id.parse()
                .map_err(|failure| error(format!("неверный идентификатор роли: {failure}")))?,
        ],
        AccessRightSubject::Principal(user) => {
            // Явное описание пользователя требует администрирования даже
            // при совпадении с текущим пользователем. Роль такой проверки не требует.
            if !privileged {
                let root: Guid = root
                    .ok_or_else(|| error("идентификатор корня конфигурации не приобретён"))?
                    .parse()
                    .map_err(|failure| {
                        error(format!("неверный идентификатор конфигурации: {failure}"))
                    })?;
                if !grants(access, &role_ids(principal)?, &root, &Right::Administration)? {
                    return Err(error(
                        "для проверки явно заданного пользователя требуется административное право",
                    ));
                }
            }
            let id: Guid = user.id().parse().map_err(|failure| {
                error(format!("неверный идентификатор пользователя: {failure}"))
            })?;
            if !access
                .users()
                .is_some_and(|users| users.iter().any(|user| user.data.id == id))
            {
                return Err(error("описание проверяемого пользователя не приобретено"));
            }
            role_ids(user)?
        }
    })
}

pub(super) fn role_ids(principal: &Principal) -> RtResult<Vec<Guid>> {
    principal
        .role_ids()
        .map(|id| {
            id.parse()
                .map_err(|failure| error(format!("неверный идентификатор роли: {failure}")))
        })
        .collect()
}

pub(super) fn grants(
    access: &AccessStore,
    roles: &[Guid],
    root: &Guid,
    right: &Right,
) -> RtResult<bool> {
    if !access.missing_rights(roles).is_empty() || !access.unreadable_rights(roles).is_empty() {
        return Err(error("данные прав проверяемых ролей не приобретены"));
    }
    Ok(access
        .rights_of(roles)
        .iter()
        .any(|rights| rights.grants(root, right)))
}

#[cfg(test)]
#[path = "configuration_rights_tests.rs"]
mod tests;
