//! `РасписаниеРегламентногоЗадания` и `РегламентныеЗадания`.

use super::*;

#[derive(Debug)]
struct JobScheduleObject {
    completion_time: RefCell<Value>,
    end_time: RefCell<Value>,
    begin_time: RefCell<Value>,
    end_date: RefCell<Value>,
    begin_date: RefCell<Value>,
    day_in_month: RefCell<Value>,
    week_day_in_month: RefCell<Value>,
    detailed_daily_schedules: RefCell<Value>,
    week_days: RefCell<Value>,
    completion_interval: RefCell<Value>,
    months: RefCell<Value>,
    repeat_pause: RefCell<Value>,
    weeks_period: RefCell<Value>,
    repeat_period_in_day: RefCell<Value>,
    days_repeat_period: RefCell<Value>,
}

pub(crate) static JOB_SCHEDULE_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-session",
    name: "РасписаниеРегламентногоЗадания",
    type_display: "Job schedule",
    type_names: &["JobSchedule"],
};

impl ObjectProtocol for JobScheduleObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &JOB_SCHEDULE_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        JOB_SCHEDULE_PROPERTIES
    }
}

pub(crate) fn scheduled_jobs(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // Регламентные задания требуют отдельного планировщика; фоновые
    // задания не являются его заменой.
    Err(RtError::DynamicError(
        "регламентные задания не поддерживаются этим хостом".into(),
    ))
}

pub(crate) fn construct_job_schedule(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    let empty_date = || RefCell::new(Value::Date(BslDate::empty()));
    let zero = || RefCell::new(Value::number_from_i64(0));
    let empty_array = || RefCell::new(Value::new_array(Vec::new()));
    Ok(Value::new_object(JobScheduleObject {
        completion_time: empty_date(),
        end_time: empty_date(),
        begin_time: empty_date(),
        end_date: empty_date(),
        begin_date: empty_date(),
        day_in_month: zero(),
        week_day_in_month: zero(),
        detailed_daily_schedules: empty_array(),
        week_days: empty_array(),
        completion_interval: zero(),
        months: empty_array(),
        repeat_pause: zero(),
        weeks_period: RefCell::new(Value::number_from_i64(1)),
        repeat_period_in_day: zero(),
        days_repeat_period: zero(),
    }))
}

fn job_schedule(receiver: &dyn ObjectProtocol) -> RtResult<&JobScheduleObject> {
    receiver
        .downcast_ref::<JobScheduleObject>()
        .ok_or(RtError::NotAnObject)
}

fn job_schedule_date(value: Value, op: &'static str) -> RtResult<Value> {
    if matches!(value, Value::Date(_)) {
        Ok(value)
    } else {
        Err(RtError::TypeError {
            expected: "Дата",
            op,
        })
    }
}

fn job_schedule_number(value: Value, op: &'static str) -> RtResult<Value> {
    if matches!(value, Value::Number(_)) {
        Ok(value)
    } else {
        Err(RtError::TypeError {
            expected: "Число",
            op,
        })
    }
}

fn job_schedule_array(value: Value, op: &'static str) -> RtResult<Value> {
    if matches!(&value, Value::Object(object) if matches!(&**object, BslObject::Array(_))) {
        Ok(value)
    } else {
        Err(RtError::TypeError {
            expected: "Массив",
            op,
        })
    }
}

macro_rules! job_schedule_property {
    ($get:ident, $set:ident, $field:ident, $check:ident, $op:literal) => {
        fn $get(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
            Ok(job_schedule(receiver)?.$field.borrow().clone())
        }

        fn $set(
            receiver: &dyn ObjectProtocol,
            value: Value,
            _: &mut CallContext<'_>,
        ) -> RtResult<()> {
            *job_schedule(receiver)?.$field.borrow_mut() = $check(value, $op)?;
            Ok(())
        }
    };
}

job_schedule_property!(
    job_schedule_completion_time,
    set_job_schedule_completion_time,
    completion_time,
    job_schedule_date,
    "РасписаниеРегламентногоЗадания.ВремяЗавершения"
);

job_schedule_property!(
    job_schedule_end_time,
    set_job_schedule_end_time,
    end_time,
    job_schedule_date,
    "РасписаниеРегламентногоЗадания.ВремяКонца"
);

job_schedule_property!(
    job_schedule_begin_time,
    set_job_schedule_begin_time,
    begin_time,
    job_schedule_date,
    "РасписаниеРегламентногоЗадания.ВремяНачала"
);

job_schedule_property!(
    job_schedule_end_date,
    set_job_schedule_end_date,
    end_date,
    job_schedule_date,
    "РасписаниеРегламентногоЗадания.ДатаКонца"
);

job_schedule_property!(
    job_schedule_begin_date,
    set_job_schedule_begin_date,
    begin_date,
    job_schedule_date,
    "РасписаниеРегламентногоЗадания.ДатаНачала"
);

job_schedule_property!(
    job_schedule_day_in_month,
    set_job_schedule_day_in_month,
    day_in_month,
    job_schedule_number,
    "РасписаниеРегламентногоЗадания.ДеньВМесяце"
);

job_schedule_property!(
    job_schedule_week_day_in_month,
    set_job_schedule_week_day_in_month,
    week_day_in_month,
    job_schedule_number,
    "РасписаниеРегламентногоЗадания.ДеньНеделиВМесяце"
);

job_schedule_property!(
    job_schedule_detailed_daily_schedules,
    set_job_schedule_detailed_daily_schedules,
    detailed_daily_schedules,
    job_schedule_array,
    "РасписаниеРегламентногоЗадания.ДетальныеРасписанияДня"
);

job_schedule_property!(
    job_schedule_week_days,
    set_job_schedule_week_days,
    week_days,
    job_schedule_array,
    "РасписаниеРегламентногоЗадания.ДниНедели"
);

job_schedule_property!(
    job_schedule_completion_interval,
    set_job_schedule_completion_interval,
    completion_interval,
    job_schedule_number,
    "РасписаниеРегламентногоЗадания.ИнтервалЗавершения"
);

job_schedule_property!(
    job_schedule_months,
    set_job_schedule_months,
    months,
    job_schedule_array,
    "РасписаниеРегламентногоЗадания.Месяцы"
);

job_schedule_property!(
    job_schedule_repeat_pause,
    set_job_schedule_repeat_pause,
    repeat_pause,
    job_schedule_number,
    "РасписаниеРегламентногоЗадания.ПаузаПовтора"
);

job_schedule_property!(
    job_schedule_weeks_period,
    set_job_schedule_weeks_period,
    weeks_period,
    job_schedule_number,
    "РасписаниеРегламентногоЗадания.ПериодНедель"
);

job_schedule_property!(
    job_schedule_repeat_period_in_day,
    set_job_schedule_repeat_period_in_day,
    repeat_period_in_day,
    job_schedule_number,
    "РасписаниеРегламентногоЗадания.ПериодПовтораВТечениеДня"
);

job_schedule_property!(
    job_schedule_days_repeat_period,
    set_job_schedule_days_repeat_period,
    days_repeat_period,
    job_schedule_number,
    "РасписаниеРегламентногоЗадания.ПериодПовтораДней"
);

pub(crate) static JOB_SCHEDULE_PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["ВремяЗавершения", "CompletionTime"],
        get: job_schedule_completion_time,
        set: Some(set_job_schedule_completion_time),
    },
    PropertyDescriptor {
        names: &["ВремяКонца", "EndTime"],
        get: job_schedule_end_time,
        set: Some(set_job_schedule_end_time),
    },
    PropertyDescriptor {
        names: &["ВремяНачала", "BeginTime"],
        get: job_schedule_begin_time,
        set: Some(set_job_schedule_begin_time),
    },
    PropertyDescriptor {
        names: &["ДатаКонца", "EndDate"],
        get: job_schedule_end_date,
        set: Some(set_job_schedule_end_date),
    },
    PropertyDescriptor {
        names: &["ДатаНачала", "BeginDate"],
        get: job_schedule_begin_date,
        set: Some(set_job_schedule_begin_date),
    },
    PropertyDescriptor {
        names: &["ДеньВМесяце", "DayInMonth"],
        get: job_schedule_day_in_month,
        set: Some(set_job_schedule_day_in_month),
    },
    PropertyDescriptor {
        names: &["ДеньНеделиВМесяце", "WeekDayInMonth"],
        get: job_schedule_week_day_in_month,
        set: Some(set_job_schedule_week_day_in_month),
    },
    PropertyDescriptor {
        names: &["ДетальныеРасписанияДня", "DetailedDailySchedules"],
        get: job_schedule_detailed_daily_schedules,
        set: Some(set_job_schedule_detailed_daily_schedules),
    },
    PropertyDescriptor {
        names: &["ДниНедели", "WeekDays"],
        get: job_schedule_week_days,
        set: Some(set_job_schedule_week_days),
    },
    PropertyDescriptor {
        names: &["ИнтервалЗавершения", "CompletionInterval"],
        get: job_schedule_completion_interval,
        set: Some(set_job_schedule_completion_interval),
    },
    PropertyDescriptor {
        names: &["Месяцы", "Months"],
        get: job_schedule_months,
        set: Some(set_job_schedule_months),
    },
    PropertyDescriptor {
        names: &["ПаузаПовтора", "RepeatPause"],
        get: job_schedule_repeat_pause,
        set: Some(set_job_schedule_repeat_pause),
    },
    PropertyDescriptor {
        names: &["ПериодНедель", "WeeksPeriod"],
        get: job_schedule_weeks_period,
        set: Some(set_job_schedule_weeks_period),
    },
    PropertyDescriptor {
        names: &["ПериодПовтораВТечениеДня", "RepeatPeriodInDay"],
        get: job_schedule_repeat_period_in_day,
        set: Some(set_job_schedule_repeat_period_in_day),
    },
    PropertyDescriptor {
        names: &["ПериодПовтораДней", "DaysRepeatPeriod"],
        get: job_schedule_days_repeat_period,
        set: Some(set_job_schedule_days_repeat_period),
    },
];
