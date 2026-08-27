# \DefaultApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**switches_get**](DefaultApi.md#switches_get) | **GET** /switches | List all switches
[**switches_switch_id_get**](DefaultApi.md#switches_switch_id_get) | **GET** /switches/{switch_id} | switches details
[**switches_switch_id_ports_port_id_delete**](DefaultApi.md#switches_switch_id_ports_port_id_delete) | **DELETE** /switches/{switch_id}/ports/{port_id} | disable port
[**switches_switch_id_ports_port_id_put**](DefaultApi.md#switches_switch_id_ports_port_id_put) | **PUT** /switches/{switch_id}/ports/{port_id} | enable port
[**switches_switch_id_setup_post**](DefaultApi.md#switches_switch_id_setup_post) | **POST** /switches/{switch_id}/setup | setup switch



## switches_get

> std::collections::HashMap<String, models::SwitchModelDetail> switches_get()
List all switches

Get all switch IDs from the inventory

### Parameters

This endpoint does not need any parameter.

### Return type

[**std::collections::HashMap<String, models::SwitchModelDetail>**](SwitchModelDetail.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## switches_switch_id_get

> Vec<models::PortRepresentation> switches_switch_id_get(switch_id)
switches details

Get information about a switch

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**switch_id** | **String** | ID of a switch | [required] |

### Return type

[**Vec<models::PortRepresentation>**](PortRepresentation.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## switches_switch_id_ports_port_id_delete

> switches_switch_id_ports_port_id_delete(port_id, switch_id)
disable port

Disable a switch port and un-assign VLAN

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**port_id** | **String** | PortID of the switch | [required] |
**switch_id** | **String** | ID of a switch | [required] |

### Return type

 (empty response body)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: Not defined

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## switches_switch_id_ports_port_id_put

> switches_switch_id_ports_port_id_put(port_id, switch_id, vlan_id)
enable port

Enable a switch port and assign a VLAN

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**port_id** | **String** | PortID of the switch | [required] |
**switch_id** | **String** | ID of a switch | [required] |
**vlan_id** | [**VlanId**](VlanId.md) | Wrapper type for different VLAN IDs potentially used for different use cases. | [required] |

### Return type

 (empty response body)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: Not defined

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## switches_switch_id_setup_post

> switches_switch_id_setup_post(switch_id, setup_data)
setup switch

setup VLANs and trunk ports

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**switch_id** | **String** | ID of a switch | [required] |
**setup_data** | [**SetupData**](SetupData.md) | The user provided input for switch setup. Contains a range of u16 representing the allowed VLAN IDs. | [required] |

### Return type

 (empty response body)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: Not defined

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

